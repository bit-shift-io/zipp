package io.bitshift.zipp

import android.Manifest
import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCallback
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothGattDescriptor
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothProfile
import android.bluetooth.BluetoothStatusCodes
import android.bluetooth.le.BluetoothLeScanner
import android.bluetooth.le.ScanCallback
import android.bluetooth.le.ScanFilter
import android.bluetooth.le.ScanResult
import android.bluetooth.le.ScanSettings
import android.content.Context
import android.content.pm.PackageManager
import android.location.Location
import android.location.LocationListener
import android.location.LocationManager
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.os.Handler
import android.os.Looper
import android.util.Log
import java.lang.ref.WeakReference
import java.util.UUID

// Everything the Rust core asks the OS for: BLE scan and GATT, GPS and Wi-Fi.
// Rust owns all data and logic; this object only drives the Android APIs and
// reports every change through the native callbacks. The `external fun`s are
// instance methods on the singleton, which is why the Rust `Java_...` exports
// take a `JObject`; the commands Rust calls back with `CallStaticVoidMethod`
// are `@JvmStatic` so they resolve as static methods on
// `io/bitshift/zipp/ZippBridge`.
object ZippBridge {
    /// The Client Characteristic Configuration descriptor shared by every GATT
    /// server; writing it turns notifications or indications on.
    private const val CCCD = "00002902-0000-1000-8000-00805f9b34fb"
    private const val REQUEST_PERMISSIONS = 1
    private const val SCAN_TIMEOUT_MS = 15_000L
    private const val CONNECT_TIMEOUT_MS = 10_000L
    private const val GATT_WATCHDOG_MS = 2_000L
    private const val LOG_TAG = "zipp"

    private var activity: WeakReference<MainActivity>? = null
    private lateinit var context: Context
    private var bluetoothManager: BluetoothManager? = null
    private var adapter: BluetoothAdapter? = null
    private var scanner: BluetoothLeScanner? = null
    private var gatt: BluetoothGatt? = null
    private var characteristics = mutableMapOf<UUID, BluetoothGattCharacteristic>()
    private var targetName = "BScooter"
    private var pendingScanTarget: String? = null
    private var scanTimeout: Runnable? = null
    private var connectTimeout: Runnable? = null
    private var locationManager: LocationManager? = null
    private var locationListener: LocationListener? = null
    private var connectivity: ConnectivityManager? = null
    private var networkCallback: ConnectivityManager.NetworkCallback? = null
    private val handler = Handler(Looper.getMainLooper())

    // Android allows one outstanding GATT operation per connection, so every
    // subscribe/read/write is queued and the next one starts only when the
    // previous callback arrives. Issuing them concurrently (as an earlier
    // version did) silently drops all but the first, which leaves notifications
    // disabled and the dashboard stuck on its defaults.
    private sealed class GattOp {
        abstract val characteristic: BluetoothGattCharacteristic

        class Enable(
            override val characteristic: BluetoothGattCharacteristic,
            val indicate: Boolean,
        ) : GattOp()

        class Read(override val characteristic: BluetoothGattCharacteristic) : GattOp()

        class Write(
            override val characteristic: BluetoothGattCharacteristic,
            val bytes: ByteArray,
            val writeType: Int,
        ) : GattOp()
    }

    // Rust calls the command methods from the native `android_main` thread,
    // while the GATT callbacks (and the watchdog) arrive on the Java UI looper,
    // so every access to the queue and its flags is guarded by `gattLock`.
    private val gattLock = Any()
    private val gattQueue = ArrayDeque<GattOp>()
    private var gattBusy = false
    private var gattWatchdog: Runnable? = null

    // Native entry points called from Rust. Declared inside the object, so they
    // are instance methods on the singleton and the Rust exports receive the
    // object as their `JObject` receiver.
    external fun nativeInit(locale: String)
    external fun nativeBack()
    external fun onScanResult(address: String, name: String, rssi: Int)
    external fun onScanState(scanning: Boolean)
    external fun onAdapterState(on: Boolean)
    external fun onConnectionState(address: String, connected: Boolean)
    external fun onNotification(uuid: String, bytes: ByteArray)
    external fun onRead(uuid: String, bytes: ByteArray)
    external fun onWifiState(connected: Boolean)
    external fun onGpsStatus(status: String)
    external fun onGpsSpeed(mps: Double)
    external fun onError(message: String)

    /// Remember the owning Activity weakly, keep the application context, and
    /// look up the Bluetooth and connectivity services. Called once from
    /// `MainActivity.onCreate` before `nativeInit`.
    @JvmStatic
    fun install(activity: MainActivity, appContext: Context) {
        this.activity = WeakReference(activity)
        context = appContext.applicationContext
        bluetoothManager = context.getSystemService(Context.BLUETOOTH_SERVICE) as BluetoothManager
        adapter = bluetoothManager?.adapter
        scanner = adapter?.bluetoothLeScanner
        connectivity = context.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
    }

    /// Ask for the runtime permissions scanning needs, skipping the granted
    /// ones. The result arrives in `MainActivity.onRequestPermissionsResult`.
    @JvmStatic
    fun requestPermissions() {
        val activity = activity?.get() ?: return
        val needed = mutableListOf<String>()
        for (permission in listOf(
            Manifest.permission.BLUETOOTH_SCAN,
            Manifest.permission.BLUETOOTH_CONNECT,
            Manifest.permission.ACCESS_FINE_LOCATION,
        )) {
            if (activity.checkSelfPermission(permission) != PackageManager.PERMISSION_GRANTED) {
                needed += permission
            }
        }
        if (needed.isNotEmpty()) {
            activity.requestPermissions(needed.toTypedArray(), REQUEST_PERMISSIONS)
        }
    }

    /// From `MainActivity.onRequestPermissionsResult`: retry the scan that was
    /// waiting on permissions and start GPS, or report the refusal.
    fun onPermissionResult(granted: Boolean) {
        if (!granted) {
            onError("Permissions denied for scanning")
            return
        }
        pendingScanTarget?.let { target ->
            pendingScanTarget = null
            startScan(target)
        }
        startGps()
    }

    private fun hasScanPermissions(): Boolean =
        context.checkSelfPermission(Manifest.permission.BLUETOOTH_SCAN) ==
            PackageManager.PERMISSION_GRANTED &&
            context.checkSelfPermission(Manifest.permission.BLUETOOTH_CONNECT) ==
            PackageManager.PERMISSION_GRANTED &&
            context.checkSelfPermission(Manifest.permission.ACCESS_FINE_LOCATION) ==
            PackageManager.PERMISSION_GRANTED

    @JvmStatic
    fun startScan(target: String) {
        targetName = target
        if (!hasScanPermissions()) {
            pendingScanTarget = target
            requestPermissions()
            return
        }
        // `onAdapterState(false)` already makes the core log the single
        // "Bluetooth is off" line (as the original app does); a second `onError` would
        // duplicate it on every reconnect tick.
        val bleAdapter = adapter ?: run {
            onAdapterState(false)
            return
        }
        if (!bleAdapter.isEnabled) {
            onAdapterState(false)
            return
        }
        onAdapterState(true)
        stopScan()
        try {
            val filter = ScanFilter.Builder().setDeviceName(target).build()
            val settings = ScanSettings.Builder()
                .setScanMode(ScanSettings.SCAN_MODE_LOW_LATENCY)
                .build()
            scanner?.startScan(listOf(filter), settings, scanCallback)
            onScanState(true)
            scanTimeout = Runnable { stopScan() }.also {
                handler.postDelayed(it, SCAN_TIMEOUT_MS)
            }
        } catch (error: Exception) {
            onError("Scan Error: $error")
        }
    }

    @JvmStatic
    fun stopScan() {
        scanTimeout?.let { handler.removeCallbacks(it) }
        scanTimeout = null
        try {
            scanner?.stopScan(scanCallback)
        } catch (_: Exception) {
        }
        onScanState(false)
    }

    /// Reconnect to a bonded device when there is one, otherwise scan for it.
    @JvmStatic
    fun reconnect(target: String) {
        val bonded = adapter?.bondedDevices?.firstOrNull { it.name == target }
        if (bonded != null) {
            connect(bonded.address)
        } else {
            startScan(target)
        }
    }

    @JvmStatic
    fun connect(address: String) {
        val device = adapter?.getRemoteDevice(address) ?: return
        stopScan()
        disconnect()
        gatt = device.connectGatt(context, false, gattCallback, BluetoothDevice.TRANSPORT_LE)
        connectTimeout = Runnable {
            onError("Connection error: timeout")
            disconnect()
        }.also { handler.postDelayed(it, CONNECT_TIMEOUT_MS) }
    }

    @JvmStatic
    fun disconnect() {
        connectTimeout?.let { handler.removeCallbacks(it) }
        connectTimeout = null
        resetGattQueue()
        characteristics.clear()
        gatt?.disconnect()
        gatt?.close()
        gatt = null
    }

    /// Write to a characteristic, without a response when it supports one.
    @JvmStatic
    fun write(service: String, characteristic: String, bytes: ByteArray) {
        if (gatt == null) return
        val char = characteristics[UUID.fromString(characteristic)] ?: return
        val withoutResponse =
            char.properties and BluetoothGattCharacteristic.PROPERTY_WRITE_NO_RESPONSE != 0
        val writeType = if (withoutResponse) {
            BluetoothGattCharacteristic.WRITE_TYPE_NO_RESPONSE
        } else {
            BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT
        }
        enqueueGatt(GattOp.Write(char, bytes, writeType))
    }

    @JvmStatic
    fun read(service: String, characteristic: String) {
        if (gatt == null) return
        val char = characteristics[UUID.fromString(characteristic)] ?: return
        enqueueGatt(GattOp.Read(char))
    }

    private fun enqueueGatt(op: GattOp) {
        synchronized(gattLock) { gattQueue.addLast(op) }
        runNextGattOp()
    }

    /// Starts the next queued operation unless another one is still in flight.
    private fun runNextGattOp() {
        val next = synchronized(gattLock) {
            if (gattBusy) return
            val gatt = gatt ?: return
            val op = gattQueue.removeFirstOrNull() ?: return
            gattBusy = true
            gatt to op
        }
        val (gatt, op) = next
        val started = try {
            when (op) {
                is GattOp.Enable -> startEnable(gatt, op)
                is GattOp.Read -> gatt.readCharacteristic(op.characteristic)
                is GattOp.Write ->
                    gatt.writeCharacteristic(op.characteristic, op.bytes, op.writeType) ==
                        BluetoothStatusCodes.SUCCESS
            }
        } catch (error: Exception) {
            onError("Command Failed: $error")
            false
        }
        if (started) {
            armGattWatchdog()
        } else {
            Log.w(LOG_TAG, "GATT op did not start: ${op.javaClass.simpleName}")
            synchronized(gattLock) { gattBusy = false }
            runNextGattOp()
        }
    }

    private fun startEnable(gatt: BluetoothGatt, op: GattOp.Enable): Boolean {
        if (!gatt.setCharacteristicNotification(op.characteristic, true)) {
            Log.w(LOG_TAG, "setCharacteristicNotification failed for ${op.characteristic.uuid}")
            return false
        }
        val descriptor = op.characteristic.getDescriptor(UUID.fromString(CCCD))
        if (descriptor == null) {
            Log.w(LOG_TAG, "no CCCD for ${op.characteristic.uuid}")
            return false
        }
        val value = if (op.indicate) {
            BluetoothGattDescriptor.ENABLE_INDICATION_VALUE
        } else {
            BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE
        }
        return gatt.writeDescriptor(descriptor, value) == BluetoothStatusCodes.SUCCESS
    }

    private fun finishGattOp() {
        synchronized(gattLock) {
            cancelGattWatchdogLocked()
            gattBusy = false
        }
        runNextGattOp()
    }

    private fun armGattWatchdog() {
        val watchdog = Runnable {
            synchronized(gattLock) {
                gattBusy = false
                gattWatchdog = null
            }
            Log.w(LOG_TAG, "GATT op timed out; continuing")
            runNextGattOp()
        }
        synchronized(gattLock) {
            cancelGattWatchdogLocked()
            gattWatchdog = watchdog
        }
        handler.postDelayed(watchdog, GATT_WATCHDOG_MS)
    }

    private fun cancelGattWatchdogLocked() {
        gattWatchdog?.let { handler.removeCallbacks(it) }
        gattWatchdog = null
    }

    private fun resetGattQueue() {
        synchronized(gattLock) {
            cancelGattWatchdogLocked()
            gattQueue.clear()
            gattBusy = false
        }
    }

    // The scan filter accepts the target name, so every result is that device.
    private val scanCallback = object : ScanCallback() {
        override fun onScanResult(callbackType: Int, result: ScanResult) {
            val name = result.scanRecord?.deviceName
                ?: result.device.name
                ?: ""
            ZippBridge.onScanResult(result.device.address, name, result.rssi)
        }

        override fun onScanFailed(errorCode: Int) {
            ZippBridge.onError("Scan Error: $errorCode")
            ZippBridge.onScanState(false)
        }
    }

    private val gattCallback = object : BluetoothGattCallback() {
        override fun onConnectionStateChange(gatt: BluetoothGatt, status: Int, newState: Int) {
            val device = gatt.device
            if (status == BluetoothGatt.GATT_SUCCESS &&
                newState == BluetoothProfile.STATE_CONNECTED
            ) {
                cancelConnectTimeout()
                gatt.discoverServices()
                ZippBridge.onConnectionState(device.address, true)
            } else if (newState == BluetoothProfile.STATE_DISCONNECTED) {
                cancelConnectTimeout()
                resetGattQueue()
                characteristics.clear()
                gatt.close()
                if (this@ZippBridge.gatt === gatt) {
                    this@ZippBridge.gatt = null
                }
                ZippBridge.onConnectionState(device.address, false)
            } else {
                ZippBridge.onError("Connection error: status $status")
            }
        }

        override fun onServicesDiscovered(gatt: BluetoothGatt, status: Int) {
            if (status != BluetoothGatt.GATT_SUCCESS) {
                ZippBridge.onError("Connection error: status $status")
                return
            }
            resetGattQueue()
            for (service in gatt.services) {
                for (characteristic in service.characteristics) {
                    characteristics[characteristic.uuid] = characteristic
                    val properties = characteristic.properties
                    val notify =
                        (properties and BluetoothGattCharacteristic.PROPERTY_NOTIFY) != 0
                    val indicate =
                        (properties and BluetoothGattCharacteristic.PROPERTY_INDICATE) != 0
                    if (notify || indicate) {
                        enqueueGatt(GattOp.Enable(characteristic, indicate && !notify))
                    }
                    if ((properties and BluetoothGattCharacteristic.PROPERTY_READ) != 0) {
                        enqueueGatt(GattOp.Read(characteristic))
                    }
                }
            }
        }

        @Suppress("DEPRECATION", "OVERRIDE_DEPRECATION")
        override fun onCharacteristicChanged(
            gatt: BluetoothGatt,
            characteristic: BluetoothGattCharacteristic,
        ) {
            ZippBridge.onNotification(characteristic.uuid.toString(), characteristic.value)
        }

        override fun onCharacteristicChanged(
            gatt: BluetoothGatt,
            characteristic: BluetoothGattCharacteristic,
            value: ByteArray,
        ) {
            ZippBridge.onNotification(characteristic.uuid.toString(), value)
        }

        override fun onDescriptorWrite(
            gatt: BluetoothGatt,
            descriptor: BluetoothGattDescriptor,
            status: Int,
        ) {
            if (status != BluetoothGatt.GATT_SUCCESS) {
                Log.w(LOG_TAG, "CCCD write failed for ${descriptor.characteristic.uuid}: $status")
            }
            finishGattOp()
        }

        @Suppress("DEPRECATION", "OVERRIDE_DEPRECATION")
        override fun onCharacteristicRead(
            gatt: BluetoothGatt,
            characteristic: BluetoothGattCharacteristic,
            status: Int,
        ) {
            if (status == BluetoothGatt.GATT_SUCCESS) {
                ZippBridge.onRead(characteristic.uuid.toString(), characteristic.value)
            } else {
                ZippBridge.onError("Command Failed: status $status")
            }
            finishGattOp()
        }

        override fun onCharacteristicRead(
            gatt: BluetoothGatt,
            characteristic: BluetoothGattCharacteristic,
            value: ByteArray,
            status: Int,
        ) {
            if (status == BluetoothGatt.GATT_SUCCESS) {
                ZippBridge.onRead(characteristic.uuid.toString(), value)
            } else {
                ZippBridge.onError("Command Failed: status $status")
            }
            finishGattOp()
        }

        override fun onCharacteristicWrite(
            gatt: BluetoothGatt,
            characteristic: BluetoothGattCharacteristic,
            status: Int,
        ) {
            if (status != BluetoothGatt.GATT_SUCCESS) {
                ZippBridge.onError("Command Failed: status $status")
            }
            finishGattOp()
        }
    }

    private fun cancelConnectTimeout() {
        connectTimeout?.let { handler.removeCallbacks(it) }
        connectTimeout = null
    }

    // GPS speed only; the status strings ("denied", "disabled", "searching",
    // "active") are the ones the core maps in `platform::android`.
    @JvmStatic
    fun startGps() {
        if (context.checkSelfPermission(Manifest.permission.ACCESS_FINE_LOCATION) !=
            PackageManager.PERMISSION_GRANTED
        ) {
            onGpsStatus("denied")
            return
        }
        val manager = context.getSystemService(Context.LOCATION_SERVICE) as LocationManager
        locationManager = manager
        if (!manager.isProviderEnabled(LocationManager.GPS_PROVIDER)) {
            onGpsStatus("disabled")
            return
        }
        val listener = object : LocationListener {
            override fun onLocationChanged(location: Location) {
                ZippBridge.onGpsStatus("active")
                ZippBridge.onGpsSpeed(location.speed.toDouble())
            }

            override fun onProviderEnabled(provider: String) {}

            override fun onProviderDisabled(provider: String) {
                ZippBridge.onGpsStatus("disabled")
            }

            @Suppress("OVERRIDE_DEPRECATION")
            @Deprecated("Deprecated in Java")
            override fun onStatusChanged(
                provider: String?,
                status: Int,
                extras: android.os.Bundle?,
            ) {
            }
        }
        locationListener = listener
        try {
            manager.requestLocationUpdates(
                LocationManager.GPS_PROVIDER,
                0L,
                0f,
                listener,
                Looper.getMainLooper(),
            )
            onGpsStatus("searching")
        } catch (error: Exception) {
            onGpsStatus("disabled")
            onError("GPS Error: $error")
        }
    }

    @JvmStatic
    fun stopGps() {
        locationListener?.let { locationManager?.removeUpdates(it) }
        locationListener = null
        locationManager = null
    }

    /// Follow the default network and report whether it is Wi-Fi.
    @JvmStatic
    fun registerWifi() {
        val manager = connectivity ?: return
        networkCallback?.let { manager.unregisterNetworkCallback(it) }
        val callback = object : ConnectivityManager.NetworkCallback() {
            override fun onAvailable(network: Network) = report(manager, network)

            override fun onCapabilitiesChanged(
                network: Network,
                caps: NetworkCapabilities,
            ) = report(manager, network)

            override fun onLost(network: Network) {
                ZippBridge.onWifiState(false)
            }
        }
        networkCallback = callback
        manager.registerDefaultNetworkCallback(callback)
        val current = manager.activeNetwork
        ZippBridge.onWifiState(
            current != null &&
                manager.getNetworkCapabilities(current)
                    ?.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) == true
        )
    }

    private fun report(manager: ConnectivityManager, network: Network) {
        val wifi = manager.getNetworkCapabilities(network)
            ?.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) == true
        onWifiState(wifi)
    }

    /// Release every OS registration; called from `MainActivity.onDestroy`.
    @JvmStatic
    fun shutdown() {
        stopScan()
        disconnect()
        stopGps()
        networkCallback?.let { connectivity?.unregisterNetworkCallback(it) }
        networkCallback = null
        activity = null
    }
}

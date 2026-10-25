// Manages Bluetooth operations using flutter_blue_plus. Handles scanning, connecting, service discovery, characteristic subscription, and command transmission with logging.
import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:permission_handler/permission_handler.dart';

import 'package:zipp/src/rust/api/protocol.dart';
import 'package:zipp/src/rust/frb_generated.dart';

class ScooterBluetoothService extends ChangeNotifier {
  // State
  String? _targetDeviceName;
  List<ScanResult> _scanResults = [];
  bool _scanning = false;
  bool _isConnecting = false;
  BluetoothDevice? _connectedDevice;
  ScooterState? _scooterState;

  // Getters
  List<ScanResult> get scanResults => _scanResults;
  bool get isScanning => _scanning;
  bool get isConnecting => _isConnecting;
  BluetoothDevice? get connectedDevice => _connectedDevice;
  ScooterState? get scooterState => _scooterState;

  // Log Stream
  final _logController = StreamController<String>.broadcast();
  Stream<String> get logStream => _logController.stream;

  // Internal
  StreamSubscription<BluetoothConnectionState>? _connectionSubscription;
  Timer? _reconnectTimer;

  // Lifecycle
  Future<void> init() async {
    // Initialize Rust
    await RustLib.init();

    // Get target name
    _targetDeviceName = getTargetDeviceName();

    // Initial State - Keep null so UI shows "Active/White" defaults until connected
    // _scooterState = await ScooterState.default_();
    notifyListeners();

    // Listen to Scan Results
    FlutterBluePlus.scanResults.listen((results) {
      _scanResults = results;
      notifyListeners();

      // Auto-connect
      if (_connectedDevice == null && !_isConnecting) {
        for (var result in results) {
          final name = result.advertisementData.localName.isNotEmpty
              ? result.advertisementData.localName
              : result.device.platformName;

          if (name == _targetDeviceName) {
            connect(result.device);
            break;
          }
        }
      }
    });

    // Listen to Scanning State
    FlutterBluePlus.isScanning.listen((state) {
      _scanning = state;
      notifyListeners();
    });

    // Start Reconnect Loop
    _startReconnectLoop();

    // Start Initial Scan
    scan();
  }

  @override
  void dispose() {
    _connectionSubscription?.cancel();
    _reconnectTimer?.cancel();
    _logController.close();
    super.dispose();
  }

  // Logic
  void _startReconnectLoop() {
    _reconnectTimer?.cancel();
    _reconnectTimer = Timer.periodic(const Duration(seconds: 5), (timer) async {
      if (_connectedDevice == null && !_isConnecting) {
        await _checkBondedDevices();
        if (!_isConnecting && !_scanning) {
          scan();
        }
      }
    });
  }

  Future<void> _checkBondedDevices() async {
    if (_targetDeviceName == null) return;
    final bonded = await FlutterBluePlus.bondedDevices;
    for (var device in bonded) {
      if (device.platformName == _targetDeviceName &&
          _connectedDevice == null &&
          !_isConnecting) {
        connect(device);
        break;
      }
    }
  }

  Future<void> scan() async {
    // Permissions
    if (Platform.isAndroid || Platform.isIOS) {
      Map<Permission, PermissionStatus> statuses = await [
        Permission.bluetoothScan,
        Permission.bluetoothConnect,
        Permission.location,
      ].request();

      if (statuses.values.any((element) => element.isDenied)) {
        _logController.add("Permissions denied for scanning");
        return;
      }
    }

    // Bluetooth check
    if (await FlutterBluePlus.adapterState.first != BluetoothAdapterState.on) {
      _logController.add("Bluetooth is off");
      return;
    }

    if (_targetDeviceName != null) {
      try {
        _logController.add("start scan");
        await FlutterBluePlus.startScan(
          timeout: const Duration(seconds: 15),
          androidUsesFineLocation: true,
          withNames: [_targetDeviceName!],
        );
      } catch (e) {
        _logController.add("Scan Error: $e");
      }
    }
  }

  Future<void> connect(BluetoothDevice device) async {
    if (_isConnecting) return;

    try {
      _isConnecting = true;
      notifyListeners();

      if (_scanning) {
        _logController.add("stop scan");
        await FlutterBluePlus.stopScan();
      }

      _logController.add("connecting: ${device.platformName}");
      await device.connect(
        autoConnect: false,
        timeout: const Duration(seconds: 10),
      );

      await _connectionSubscription?.cancel();
      _connectionSubscription = device.connectionState.listen((state) {
        if (state == BluetoothConnectionState.disconnected) {
          _logController.add("disconnected from device");
          _connectedDevice = null;
          _isConnecting = false;
          notifyListeners();
        } else if (state == BluetoothConnectionState.connected) {
          _logController.add("connected: ${device.platformName}");
        }
      });

      _connectedDevice = device;
      _isConnecting = false;
      notifyListeners();

      // Discover Services
      final services = await device.discoverServices();
      for (var service in services) {
        for (var characteristic in service.characteristics) {
          if (characteristic.properties.notify ||
              characteristic.properties.indicate) {
            await characteristic.setNotifyValue(true);
            characteristic.onValueReceived.listen((value) async {
              // Ensure state exists before parsing
              _scooterState ??= await ScooterState.default_();

              final result = parseCharacteristicData(
                state: _scooterState!,
                uuid: characteristic.uuid.toString(),
                data: value,
              );

              if (result.log != null) {
                _logController.add(result.log!);
              }

              _scooterState = result.state;
              notifyListeners();
            });
          }
          if (characteristic.properties.read) {
            final value = await characteristic.read();
            // Ensure state exists before parsing
            _scooterState ??= await ScooterState.default_();

            final result = parseCharacteristicData(
              state: _scooterState!,
              uuid: characteristic.uuid.toString(),
              data: value,
            );
            _scooterState = result.state;
            notifyListeners();
          }
        }
      }
    } catch (e) {
      _logController.add("Connection error: $e");
      _isConnecting = false;
      notifyListeners();
    }
  }

  Future<void> sendCommand(ScooterCommand command) async {
    // Add empty line for SetUart to match Java behavior
    command.maybeWhen(setUart: (_) => _logController.add(""), orElse: () {});

    final logMessage = command.when(
      togglePower: () => "> sending command: TogglePower",
      toggleAlarm: () => "> sending command: ToggleAlarm",
      toggleSport: () => "> sending command: ToggleSport",
      toggleLights: () => "> sending command: ToggleLights",
      cruiseUp: () => "> sending command: CruiseUp",
      cruiseDown: () => "> sending command: CruiseDown",
      horn: () => "> sending command: Horn",
      setUart: (s) => "> $s",
    );
    _logController.add(logMessage);

    if (_connectedDevice == null) {
      _logController.add("Not connected");
      return;
    }

    _scooterState ??= await ScooterState.default_();

    final action = getCommandAction(
      command: command,
      currentState: _scooterState!,
    );

    try {
      final services = await _connectedDevice!.discoverServices();
      final service = services.firstWhere(
        (s) => s.uuid == Guid(action.serviceUuid),
      );
      final char = service.characteristics.firstWhere(
        (c) => c.uuid == Guid(action.characteristicUuid),
      );

      // Determine write type
      // Prefer WriteWithoutResponse if available (common for UART)
      bool withoutResponse = char.properties.writeWithoutResponse;

      await char.write(action.bytes, withoutResponse: withoutResponse);
    } catch (e) {
      _logController.add("Command Failed: $e");
    }
  }

  void clearLogs() {
    // This is purely for the UI to handle, as we stream logs.
    // We could implement a "clear" event if we wanted, but the UI clears its own list.
  }
}

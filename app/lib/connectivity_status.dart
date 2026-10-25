// A collapsible status widget. Shows current connection state (Connected, Scanning, Disconnected) and expands to reveal the BleScannerList.
import 'package:flutter/material.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:zipp/i18n/strings.g.dart';
import 'ble_scanner_list.dart';

class ConnectivityStatus extends StatelessWidget {
  final BluetoothDevice? connectedDevice;
  final List<ScanResult> scanResults;
  final bool isConnecting;
  final bool isScanning;
  final VoidCallback onScan;
  final Function(BluetoothDevice) onConnect;
  final Function(BluetoothDevice) onDisconnect;
  final VoidCallback onGoToScanner;

  const ConnectivityStatus({
    super.key,
    required this.connectedDevice,
    required this.scanResults,
    required this.isConnecting,
    required this.isScanning,
    required this.onScan,
    required this.onConnect,
    required this.onDisconnect,
    required this.onGoToScanner,
  });

  @override
  Widget build(BuildContext context) {
    String statusText = "";
    Color statusColor = Colors.grey;
    IconData statusIcon = Icons.bluetooth_disabled;

    if (connectedDevice != null) {
      statusText = connectedDevice!.platformName.isNotEmpty
          ? connectedDevice!.platformName
          : t.scanner.unknown;
      statusColor = const Color(0xFFCCCCCC);
      statusIcon = Icons.bluetooth_connected;
    } else if (isConnecting) {
      statusText = t.scanner.connecting;
      statusColor = const Color(0xFFCCCCCC);
      statusIcon = Icons.bluetooth_searching;
    } else if (isScanning) {
      statusText = t.scanner.scanning;
      statusColor = const Color(0xFFCCCCCC);
      statusIcon = Icons.bluetooth_searching;
    } else {
      statusText = t.scanner.disconnected;
      statusColor = const Color(0xFFCCCCCC);
      statusIcon = Icons.bluetooth_disabled;
    }

    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        InkWell(
          onTap: onGoToScanner,
          child: Container(
            padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 16),
            color: Colors.black,
            child: Row(
              children: [
                Icon(statusIcon, color: statusColor, size: 32),
                const SizedBox(width: 16),
                Expanded(
                  child: Column(
                    mainAxisAlignment: MainAxisAlignment.center,
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        statusText,
                        style: TextStyle(
                          color: statusColor,
                          fontSize: 20,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                      if (connectedDevice != null)
                        Text(
                          connectedDevice!.remoteId.str,
                          style: const TextStyle(
                            color: Colors.white54,
                            fontSize: 12,
                          ),
                        ),
                    ],
                  ),
                ),
                IconButton(
                  icon: Icon(
                    isScanning ? Icons.stop : Icons.search,
                    color: const Color(0xFFCCCCCC),
                    size: isScanning ? 32 : 28,
                  ),
                  onPressed: onScan,
                ),
              ],
            ),
          ),
        ),
        const Divider(height: 1, color: Colors.white24),
        // Scanner List integrated here
        Flexible(
          child: BleScannerList(
            connectedDevice: connectedDevice,
            scanResults: scanResults,
            isScanning: isScanning,
            onScan: onScan,
            onConnect: onConnect,
            onDisconnect: onDisconnect,
          ),
        ),
      ],
    );
  }
}

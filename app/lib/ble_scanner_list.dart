// Displays a list of discovered Bluetooth devices. Allows connecting to a specific device or disconnecting from the current one.
import 'package:flutter/material.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:zipp/i18n/strings.g.dart';

class BleScannerList extends StatelessWidget {
  final BluetoothDevice? connectedDevice;
  final List<ScanResult> scanResults;
  final bool isScanning;
  final VoidCallback onScan;
  final Function(BluetoothDevice) onConnect;
  final Function(BluetoothDevice) onDisconnect;

  const BleScannerList({
    super.key,
    required this.connectedDevice,
    required this.scanResults,
    required this.isScanning,
    required this.onScan,
    required this.onConnect,
    required this.onDisconnect,
  });

  @override
  Widget build(BuildContext context) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Flexible(
          child: ListView(
            shrinkWrap: true,
            padding: const EdgeInsets.symmetric(horizontal: 8),
            children: [
              if (connectedDevice != null)
                Card(
                  color: Colors.white10,
                  child: ListTile(
                    leading: const Icon(
                      Icons.bluetooth_connected,
                      color: Color(0xFFCCCCCC),
                    ),
                    title: Text(
                      connectedDevice!.platformName.isNotEmpty
                          ? connectedDevice!.platformName
                          : t.scanner.unknown,
                      style: const TextStyle(fontSize: 18),
                    ),
                    trailing: TextButton(
                      onPressed: () => onDisconnect(connectedDevice!),
                      child: Text(
                        t.scanner.disconnect,
                        style: const TextStyle(
                          color: Color(0xFFCCCCCC),
                          fontSize: 16,
                        ),
                      ),
                    ),
                  ),
                ),

              ...scanResults.map((result) {
                final name = result.advertisementData.localName.isNotEmpty
                    ? result.advertisementData.localName
                    : result.device.platformName;
                final isConnected = result.device == connectedDevice;
                if (isConnected) return const SizedBox.shrink();

                return ListTile(
                  leading: const Icon(Icons.bluetooth),
                  title: Text(
                    name.isNotEmpty ? name : t.scanner.unknown,
                    style: const TextStyle(fontSize: 18),
                  ),
                  subtitle: Text(
                    result.device.remoteId.str,
                    style: const TextStyle(fontSize: 14),
                  ),
                  onTap: () => onConnect(result.device),
                );
              }),
              if (scanResults.isEmpty && !isScanning && connectedDevice == null)
                const Padding(
                  padding: EdgeInsets.all(16.0),
                  child: Center(
                    child: Text(
                      "No devices found",
                      style: TextStyle(color: Colors.grey, fontSize: 16),
                    ),
                  ),
                ),
            ],
          ),
        ),
      ],
    );
  }
}

// The primary screen widget. Combines the ScooterDashboard (top) and ConnectivityStatus (bottom) into a single vertical layout.
import 'package:flutter/material.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:zipp/src/rust/api/protocol.dart';
import 'dashboard.dart';
import 'connectivity_status.dart';
import 'gps_service.dart';

class MainPage extends StatelessWidget {
  final ScooterState? scooterState;
  final double? gpsSpeed;
  final GpsStatus gpsStatus;
  final BluetoothDevice? connectedDevice;
  final List<ScanResult> scanResults;
  final bool isConnecting;
  final bool isScanning;
  final VoidCallback onScan;
  final Function(BluetoothDevice) onConnect;
  final Function(BluetoothDevice) onDisconnect;
  final Function(ScooterCommand) onSendCommand;
  final VoidCallback onGoToScanner;

  const MainPage({
    super.key,
    required this.scooterState,
    this.gpsSpeed,
    required this.gpsStatus,
    required this.connectedDevice,
    required this.scanResults,
    required this.isConnecting,
    required this.isScanning,
    required this.onScan,
    required this.onConnect,
    required this.onDisconnect,
    required this.onSendCommand,
    required this.onGoToScanner,
  });

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        // ... dashboard ...
        ScooterDashboard(
          state: scooterState,
          gpsSpeed: gpsSpeed,
          gpsStatus: gpsStatus,
          isConnecting: isConnecting,
          isScanning: isScanning,
          onSendCommand: onSendCommand,
        ),

        //const Divider(height: 2, color: Colors.white54, thickness: 2),

        // 2. Connectivity Status & Scanner (Bottom Section)
        Expanded(
          child: Container(
            color: Colors.black,
            alignment: Alignment.topCenter,
            child: ConnectivityStatus(
              connectedDevice: connectedDevice,
              scanResults: scanResults,
              isConnecting: isConnecting,
              isScanning: isScanning,
              onScan: onScan,
              onConnect: onConnect,
              onDisconnect: onDisconnect,
              onGoToScanner: onGoToScanner,
            ),
          ),
        ),
      ],
    );
  }
}

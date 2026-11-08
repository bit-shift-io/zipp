// Application entry point. Initializes services (Bluetooth, Logger, GPS), sets up the main PageView navigation (Dashboard vs Log), and configures global app theme/localization.
import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:zipp/src/rust/api/protocol.dart';
import 'logger_service.dart';
import 'main_page.dart';
import 'log_page.dart';
import 'bluetooth_service.dart';
import 'gps_service.dart';

import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:zipp/i18n/strings.g.dart';

void main() {
  WidgetsFlutterBinding.ensureInitialized();
  LocaleSettings.useDeviceLocale(); // Initialize i18n
  //LocaleSettings.setLocale(AppLocale.zh); // chinese test
  runApp(TranslationProvider(child: const MyApp())); // Wrap app
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Zipp',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(
          seedColor: Colors.blue,
          brightness: Brightness.dark,
        ),
        useMaterial3: true,
      ),
      locale: TranslationProvider.of(context).flutterLocale, // use provider
      supportedLocales: AppLocaleUtils.supportedLocales,
      localizationsDelegates: GlobalMaterialLocalizations.delegates,
      themeMode: ThemeMode.dark,
      home: const MainAppShell(title: 'Zipp - Scooter Console'),
    );
  }
}

class MainAppShell extends StatefulWidget {
  const MainAppShell({super.key, required this.title});
  final String title;

  @override
  State<MainAppShell> createState() => _MainAppShellState();
}

class _MainAppShellState extends State<MainAppShell> {
  final ScooterBluetoothService _bluetoothService = ScooterBluetoothService();
  final LoggerService _loggerService = LoggerService();
  final GpsService _gpsService = GpsService();

  final ScrollController _logScrollController = ScrollController();
  final TextEditingController _uartController = TextEditingController();
  final PageController _pageController = PageController();

  StreamSubscription? _logSubscription;

  @override
  void initState() {
    super.initState();

    // Initialize Service
    _bluetoothService.init();

    // Listen for state changes to rebuild UI
    _bluetoothService.addListener(_onServiceUpdate);
    _loggerService.addListener(_onServiceUpdate);
    _gpsService.addListener(_onServiceUpdate);

    _gpsService.init();

    // Pipe Bluetooth logs to Logger
    _logSubscription = _bluetoothService.logStream.listen((message) {
      _loggerService.add(message);
      // Auto-scroll
      if (mounted) {
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (_logScrollController.hasClients) {
            _logScrollController.jumpTo(
              _logScrollController.position.maxScrollExtent,
            );
          }
        });
      }
    });
  }

  void _onServiceUpdate() {
    if (mounted) {
      setState(() {});
    }
  }

  @override
  void dispose() {
    _bluetoothService.removeListener(_onServiceUpdate);
    _loggerService.removeListener(_onServiceUpdate);
    _gpsService.removeListener(_onServiceUpdate);

    _bluetoothService.dispose();
    _loggerService.dispose();
    _gpsService.dispose();

    _logSubscription?.cancel();
    _logScrollController.dispose();
    _uartController.dispose();
    _pageController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: Colors.black,
      body: SafeArea(
        child: PageView(
          physics: const NeverScrollableScrollPhysics(),
          controller: _pageController,
          children: [
            MainPage(
              scooterState: _bluetoothService.scooterState,
              gpsSpeed: _gpsService.currentSpeed,
              gpsStatus: _gpsService.status,
              connectedDevice: _bluetoothService.connectedDevice,
              scanResults: _bluetoothService.scanResults,
              isConnecting: _bluetoothService.isConnecting,
              isScanning: _bluetoothService.isScanning,
              onScan: _bluetoothService.scan,
              onConnect: (device) => _bluetoothService.connect(device),
              onDisconnect: (device) => device.disconnect(),
              onSendCommand: _bluetoothService.sendCommand,
              onGoToScanner: () => _pageController.animateToPage(
                1,
                duration: const Duration(milliseconds: 300),
                curve: Curves.easeInOut,
              ),
            ),
            LogPage(
              logs: _loggerService.logs,
              logScrollController: _logScrollController,
              uartController: _uartController,
              onSendCommand: _bluetoothService.sendCommand,
              onClearLogs: _loggerService.clear,
              onBack: () => _pageController.animateToPage(
                0,
                duration: const Duration(milliseconds: 300),
                curve: Curves.easeInOut,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

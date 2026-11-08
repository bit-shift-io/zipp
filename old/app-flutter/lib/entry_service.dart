import 'dart:async';
import 'dart:io' show Platform; // Import Platform to check OS

import 'package:connectivity_plus/connectivity_plus.dart';
import 'package:http/http.dart' as http;
import 'package:flutter/foundation.dart';

class EntryService {
  static final EntryService _instance = EntryService._internal();
  factory EntryService() => _instance;
  EntryService._internal();

  bool _isPending = false;
  StreamSubscription<List<ConnectivityResult>>? _subscription;

  Future<void> triggerEntry() async {
    // 1. Check Platform
    // If we are NOT on a mobile OS (Android or iOS), send immediately and return.
    bool isMobile = Platform.isAndroid || Platform.isIOS;

    if (!isMobile) {
      debugPrint("Desktop detected. Bypassing Wi-Fi check.");
      await _sendRequest();
      return;
    }

    // 2. Mobile Logic: Check for Wi-Fi
    if (_isPending) {
      debugPrint("Entry request already pending...");
      return;
    }

    final connectivityResult = await Connectivity().checkConnectivity();

    // Check if Wi-Fi is currently active
    if (connectivityResult.contains(ConnectivityResult.wifi)) {
      await _sendRequest();
    } else {
      debugPrint("Wi-Fi not connected on mobile. Waiting for connection...");
      _isPending = true;

      // Cancel any existing subscription just in case
      _cancelSubscription();

      // Listen for network changes
      _subscription = Connectivity().onConnectivityChanged.listen((results) {
        if (results.contains(ConnectivityResult.wifi)) {
          debugPrint("Wi-Fi connected! Sending entry request...");
          _sendRequest();
          // Important: Stop listening once the job is done
          _cancelSubscription();
        }
      });
    }
  }

  Future<void> _sendRequest() async {
    try {
      final url = Uri.parse("http://192.168.1.3:1880/entry");
      final response = await http.get(url);
      debugPrint("Entry request sent. Status: ${response.statusCode}");
    } catch (e) {
      debugPrint("Error sending entry request: $e");
    } finally {
      // Reset pending state regardless of success or failure
      _isPending = false;
    }
  }

  void _cancelSubscription() {
    _subscription?.cancel();
    _subscription = null;
  }
}

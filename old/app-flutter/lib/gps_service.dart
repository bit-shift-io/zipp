// Manages GPS location updates. Provides current speed and status (Active, Searching) to the dashboard.
import 'dart:async';
import 'dart:io';
import 'package:flutter/foundation.dart';
import 'package:geolocator/geolocator.dart';

enum GpsStatus { disabled, denied, searching, active }

class GpsService extends ChangeNotifier {
  double? _currentSpeed; // in km/h
  GpsStatus _status = GpsStatus.disabled;
  StreamSubscription<Position>? _positionSubscription;

  double? get currentSpeed => _currentSpeed;
  GpsStatus get status => _status;

  Future<void> init() async {
    // Check if platform is supported (geolocator does not officially support Linux in the base package)
    if (!kIsWeb && Platform.isLinux) {
      debugPrint("GPS not supported on Linux, skipping...");
      _status = GpsStatus.disabled;
      notifyListeners();
      return;
    }

    bool serviceEnabled;
    LocationPermission permission;

    // Test if location services are enabled.
    serviceEnabled = await Geolocator.isLocationServiceEnabled();
    if (!serviceEnabled) {
      _status = GpsStatus.disabled;
      notifyListeners();
      return;
    }

    permission = await Geolocator.checkPermission();
    if (permission == LocationPermission.denied) {
      permission = await Geolocator.requestPermission();
      if (permission == LocationPermission.denied) {
        _status = GpsStatus.denied;
        notifyListeners();
        return;
      }
    }

    if (permission == LocationPermission.deniedForever) {
      _status = GpsStatus.denied;
      notifyListeners();
      return;
    }

    _status = GpsStatus.searching;
    notifyListeners();

    // Start listening
    const locationSettings = LocationSettings(
      accuracy: LocationAccuracy.bestForNavigation,
      distanceFilter: 0,
    );

    _positionSubscription =
        Geolocator.getPositionStream(locationSettings: locationSettings).listen(
          (Position position) {
            // position.speed is in m/s, convert to km/h
            _currentSpeed = position.speed * 3.6;

            if (_status != GpsStatus.active) {
              _status = GpsStatus.active;
            }
            notifyListeners();
          },
          onError: (error) {
            debugPrint("GPS Error: $error");
            _status = GpsStatus.disabled;
            notifyListeners();
          },
        );
  }

  void stop() {
    _positionSubscription?.cancel();
    _currentSpeed = null;
    _status = GpsStatus.disabled;
    notifyListeners();
  }

  @override
  void dispose() {
    stop();
    super.dispose();
  }
}

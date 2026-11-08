// A centralized logging service. buffers log messages with timestamps and notifies listeners (UI) of updates.
import 'package:flutter/foundation.dart';

class LoggerService extends ChangeNotifier {
  final List<String> _logs = [];
  static const int _maxLogs = 200;

  List<String> get logs => List.unmodifiable(_logs);

  void add(String message) {
    final time = DateTime.now();
    final timestamp =
        "${time.hour.toString().padLeft(2, '0')}:${time.minute.toString().padLeft(2, '0')}";
    _logs.add("$timestamp $message");

    if (_logs.length > _maxLogs) {
      _logs.removeAt(0);
    }

    notifyListeners();
  }

  void clear() {
    _logs.clear();
    notifyListeners();
  }
}

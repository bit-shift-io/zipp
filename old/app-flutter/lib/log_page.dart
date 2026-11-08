// The secondary screen widget. Hosts the LogTerminal for viewing application logs and sending raw UART commands.
import 'package:flutter/material.dart';
import 'package:zipp/src/rust/api/protocol.dart';
import 'package:zipp/i18n/strings.g.dart';
import 'log.dart';

class LogPage extends StatelessWidget {
  final List<String> logs;
  final ScrollController logScrollController;
  final TextEditingController uartController;
  final Function(ScooterCommand) onSendCommand;
  final VoidCallback onClearLogs;
  final VoidCallback onBack;

  const LogPage({
    super.key,
    required this.logs,
    required this.logScrollController,
    required this.uartController,
    required this.onSendCommand,
    required this.onClearLogs,
    required this.onBack,
  });

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        // Top section: Title
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
          child: Row(
            children: [
              IconButton(
                icon: const Icon(Icons.arrow_back_ios),
                onPressed: onBack,
              ),
              const SizedBox(width: 8),
              Text(
                t.scanner.title,
                style: const TextStyle(
                  fontSize: 28,
                  fontWeight: FontWeight.bold,
                ),
              ),
            ],
          ),
        ),

        const Divider(height: 1, color: Colors.white24),

        // Logs
        Expanded(
          child: LogTerminal(
            logs: logs,
            scrollController: logScrollController,
            uartController: uartController,
            onSendCommand: onSendCommand,
            onClearLogs: onClearLogs,
          ),
        ),
      ],
    );
  }
}

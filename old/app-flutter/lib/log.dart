// A terminal-like widget. Renders a scrolling list of logs and provides a text input field for sending manual commands to the scooter.
import 'package:flutter/material.dart';
import 'package:zipp/src/rust/api/protocol.dart';
import 'package:zipp/i18n/strings.g.dart';

class LogTerminal extends StatelessWidget {
  final List<String> logs;
  final ScrollController scrollController;
  final TextEditingController uartController;
  final Function(ScooterCommand) onSendCommand;
  final VoidCallback onClearLogs;

  const LogTerminal({
    super.key,
    required this.logs,
    required this.scrollController,
    required this.uartController,
    required this.onSendCommand,
    required this.onClearLogs,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      color: Colors.black,
      child: Column(
        children: [
          Expanded(
            child: Stack(
              children: [
                ListView.builder(
                  controller: scrollController,
                  padding: const EdgeInsets.all(10),
                  itemCount: logs.length,
                  itemBuilder: (context, index) {
                    return Text(
                      logs[index],
                      style: const TextStyle(
                        color: Color(0xFFCCCCCC),
                        fontSize: 16,
                        height: 1.2,
                      ),
                    );
                  },
                ),
                Positioned(
                  top: 10,
                  right: 10,
                  child: Container(
                    decoration: BoxDecoration(
                      color: const Color(0xFF333333),
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: InkWell(
                      onTap: onClearLogs,
                      child: Padding(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 12,
                          vertical: 8,
                        ),
                        child: Text(
                          t.scanner.clear,
                          style: const TextStyle(
                            color: Colors.white,
                            fontSize: 16,
                          ),
                        ),
                      ),
                    ),
                  ),
                ),
              ],
            ),
          ),
          Container(
            height: 60,
            padding: const EdgeInsets.symmetric(horizontal: 10),
            decoration: const BoxDecoration(
              border: Border(top: BorderSide(color: Colors.white24, width: 1)),
            ),
            child: Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: uartController,
                    style: const TextStyle(color: Colors.white, fontSize: 18),
                    decoration: const InputDecoration(
                      hintText: "...",
                      hintStyle: TextStyle(color: Colors.white24, fontSize: 20),
                      border: InputBorder.none,
                      isDense: true,
                    ),
                    onSubmitted: (val) {
                      if (val.isNotEmpty) {
                        onSendCommand(ScooterCommand.setUart(val));
                        uartController.clear();
                      }
                    },
                  ),
                ),
                Container(
                  color: const Color(0xFF333333),
                  child: IconButton(
                    icon: const Icon(
                      Icons.chevron_right,
                      color: Colors.white,
                      size: 30,
                    ),
                    onPressed: () {
                      final val = uartController.text;
                      if (val.isNotEmpty) {
                        onSendCommand(ScooterCommand.setUart(val));
                        uartController.clear();
                      }
                    },
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

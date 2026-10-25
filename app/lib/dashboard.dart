// Displays the scooter's dashboard UI. Visualizes state (speed, battery, clock, temperature) and provides control buttons (Power, Lights, Cruise, Entry).
import 'package:flutter/material.dart';
import 'package:zipp/src/rust/api/protocol.dart';
import 'package:zipp/i18n/strings.g.dart';
import 'package:zipp/entry_service.dart';
import 'gps_service.dart';

class ScooterDashboard extends StatelessWidget {
  final ScooterState? state;
  final double? gpsSpeed;
  final GpsStatus gpsStatus;
  final bool isConnecting;
  final bool isScanning;
  final Function(ScooterCommand) onSendCommand;

  const ScooterDashboard({
    super.key,
    required this.state,
    this.gpsSpeed,
    required this.gpsStatus,
    required this.isConnecting,
    required this.isScanning,
    required this.onSendCommand,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      height: 480, // Fixed height for dashboard density
      //color: const Color(0xFF1A1A1A),
      child: Column(
        children: [
          _buildRow1Buttons(),
          // const Divider(height: 1, color: Colors.white24, thickness: 2),
          _buildRow2Stats(),
          const Divider(height: 1, color: Colors.white24, thickness: 2),
          Expanded(child: _buildRow3MainContent()),
          const Divider(height: 1, color: Colors.white24, thickness: 2),
          _buildRow4BottomStats(),
          //const Divider(height: 1, color: Colors.white24, thickness: 2),
          _buildRow5Buttons(),
        ],
      ),
    );
  }

  Widget _buildRow1Buttons() {
    return SizedBox(
      height: 90,
      child: Row(
        children: [
          Expanded(
            child: _buildDashboardButton(
              t.dashboard.power,
              state?.powerOn ?? false,
              () => onSendCommand(const ScooterCommand.togglePower()),
            ),
          ),
          Expanded(
            child: _buildDashboardButton(
              t.dashboard.alarm,
              state?.alarmOn ?? false,
              () => onSendCommand(const ScooterCommand.toggleAlarm()),
            ),
          ),
          Expanded(
            child: _buildDashboardButton(
              t.dashboard.lights,
              state?.lightsOn ?? false,
              () => onSendCommand(const ScooterCommand.toggleLights()),
            ),
          ),
          Expanded(
            child: _buildDashboardButton(
              t.dashboard.entry,
              false,
              () => EntryService().triggerEntry(),
              isAction: true,
            ),
          ),
        ],
      ),
    );
  }

  String _val(String? v, String def) {
    return (v == null || v.isEmpty) ? def : v;
  }

  Widget _buildRow2Stats() {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4, horizontal: 10),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Expanded(
            child: Align(
              alignment: Alignment.centerLeft,
              child: _buildStatText(
                "${_val(state?.temperature, "00")}${t.dashboard.units.celsius}",
                fontSize: 35,
              ),
            ),
          ),
          _buildClockWidget(),
          Expanded(
            child: Align(
              alignment: Alignment.centerRight,
              child: _buildStatText(
                "${_val(state?.batteryLevel, "0")}%",
                fontSize: 35,
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildRow3MainContent() {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 10),
      child: Row(
        children: [
          // Left: Indicators
          SizedBox(
            width: 100,
            child: Column(
              mainAxisAlignment: MainAxisAlignment.spaceEvenly,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                _buildIndicatorLabel(
                  t.dashboard.brake,
                  state?.brakeActive ?? true,
                ),
                _buildIndicatorLabel(
                  t.dashboard.cruise,
                  state == null || (state!.cruiseLevel > 0),
                ),
                _buildIndicatorLabel(
                  t.dashboard.park,
                  state?.parkBrakeActive ?? true,
                ),
              ],
            ),
          ),

          // Center: Speed
          Expanded(
            child: Column(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Padding(
                  padding: const EdgeInsets.only(top: 12.0),
                  child: Row(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      if (gpsStatus == GpsStatus.active)
                        const Icon(
                          Icons.gps_fixed,
                          size: 16,
                          color: Color(0xFFCCCCCC),
                        ),
                      if (gpsStatus == GpsStatus.searching)
                        const Icon(
                          Icons.gps_not_fixed,
                          size: 16,
                          color: Color(0xFFCCCCCC),
                        ),
                      if (gpsStatus == GpsStatus.active ||
                          gpsStatus == GpsStatus.searching)
                        const SizedBox(width: 4),
                      if (gpsStatus == GpsStatus.active ||
                          gpsStatus == GpsStatus.searching)
                        const Text(
                          "GPS",
                          style: TextStyle(
                            fontSize: 12,
                            color: Color(0xFFCCCCCC),
                          ),
                        ),
                    ],
                  ),
                ),
                Row(
                  mainAxisAlignment: MainAxisAlignment.center,
                  crossAxisAlignment: CrossAxisAlignment.baseline,
                  textBaseline: TextBaseline.alphabetic,
                  children: [
                    Text(
                      gpsStatus == GpsStatus.active && gpsSpeed != null
                          ? gpsSpeed!.toStringAsFixed(0).padLeft(2, '0')
                          : _val(state?.speed, "00"),
                      style: const TextStyle(
                        fontSize: 100,
                        fontWeight: FontWeight.normal,
                        color: Color(0xFFCCCCCC),
                        height: 1.0,
                      ),
                    ),
                    Text(
                      " ${t.dashboard.units.kmh}",
                      style: const TextStyle(
                        fontSize: 24,
                        color: Color(0xFFCCCCCC),
                      ),
                    ),
                  ],
                ),
                Padding(
                  padding: const EdgeInsets.only(bottom: 8.0),
                  child: Text(
                    "${_val(state?.throttleLevel, "0000")} ${t.dashboard.units.mv}",
                    style: const TextStyle(
                      fontSize: 24,
                      color: Color(0xFFCCCCCC),
                    ),
                  ),
                ),
              ],
            ),
          ),

          // Right: Cruise Bar Chart (5 bars)
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 10.0),
            child: SizedBox(
              width: 100, // Increased to fit max width of 95
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: List.generate(5, (index) {
                  final level = 5 - index;
                  final isActive = (state?.cruiseLevel ?? 5) >= level;
                  return Expanded(
                    child: Align(
                      alignment: Alignment.centerRight,
                      child: Container(
                        margin: const EdgeInsets.symmetric(vertical: 4),
                        width: 20.0 + (level * 15.0),
                        height: double.infinity,
                        decoration: BoxDecoration(
                          color: isActive
                              ? const Color(0xFFCCCCCC)
                              : const Color(0xFF333333),
                          borderRadius: BorderRadius.circular(100),
                        ),
                      ),
                    ),
                  );
                }),
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildRow4BottomStats() {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8, horizontal: 10),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Expanded(
            child: Align(
              alignment: Alignment.centerLeft,
              child: _buildStatText(
                "${_val(state?.odometer, "0000")} ${t.dashboard.units.km}",
                fontSize: 26,
              ),
            ),
          ),
          _buildStatText(
            "${_val(state?.batteryPower, "0000")} ${t.dashboard.units.watts}",
            fontSize: 26,
          ),
          Expanded(
            child: Align(
              alignment: Alignment.centerRight,
              child: _buildStatText(
                "${_val(state?.clockHours, "00")}:${_val(state?.clockMinutes, "00")}",
                fontSize: 26,
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildRow5Buttons() {
    return SizedBox(
      height: 90,
      child: Row(
        children: [
          Expanded(
            child: _buildDashboardButton(
              t.dashboard.sport,
              state?.sportOn ?? false,
              () => onSendCommand(const ScooterCommand.toggleSport()),
            ),
          ),
          Expanded(
            child: _buildDashboardButton(
              t.dashboard.cruise_down,
              false,
              () => onSendCommand(const ScooterCommand.cruiseDown()),
              isAction: true,
            ),
          ),
          Expanded(
            child: _buildDashboardButton(
              t.dashboard.cruise_up,
              false,
              () => onSendCommand(const ScooterCommand.cruiseUp()),
              isAction: true,
            ),
          ),
          Expanded(
            child: _buildDashboardButton(
              t.dashboard.horn,
              false,
              () => onSendCommand(const ScooterCommand.horn()),
              isAction: true,
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildDashboardButton(
    String label,
    bool isOn,
    VoidCallback onTap, {
    bool isAction = false,
  }) {
    return Padding(
      padding: const EdgeInsets.all(4.0),

      child: Material(
        color: Colors.transparent,

        child: InkWell(
          onTap: onTap,

          borderRadius: BorderRadius.circular(12),

          child: Container(
            decoration: BoxDecoration(
              color: (!isAction && isOn)
                  ? const Color(0xFF333333)
                  : Colors.transparent,

              borderRadius: BorderRadius.circular(12),

              border: Border.all(color: Colors.white24, width: 2),
            ),

            alignment: Alignment.center,

            padding: const EdgeInsets.all(8.0),

            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,

              children: [
                Text(
                  label,

                  style: const TextStyle(
                    color: Colors.white,

                    fontSize: 20,

                    fontWeight: FontWeight.normal,
                  ),

                  textAlign: TextAlign.center,
                ),

                if (!isAction)
                  Text(
                    isOn
                        ? t.dashboard.indicators.on
                        : t.dashboard.indicators.off,

                    style: const TextStyle(color: Colors.white, fontSize: 20),
                  ),
              ],
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildStatText(String text, {double fontSize = 26}) {
    return Text(
      text,
      style: TextStyle(
        fontSize: fontSize,
        color: const Color(0xFFCCCCCC),
        fontWeight: FontWeight.normal,
      ),
    );
  }

  Widget _buildIndicatorLabel(String label, bool isActive) {
    return Text(
      label,
      style: TextStyle(
        fontSize: 26,
        color: isActive ? Colors.white : const Color(0xFF333333),
        fontWeight: FontWeight.normal,
      ),
    );
  }

  Widget _buildClockWidget() {
    return StreamBuilder(
      stream: Stream.periodic(const Duration(seconds: 1)),
      builder: (context, snapshot) {
        final now = DateTime.now();
        final hours = now.hour > 12
            ? now.hour - 12
            : (now.hour == 0 ? 12 : now.hour);
        final ampm = now.hour >= 12 ? "pm" : "am";
        final timeStr = "$hours:${now.minute.toString().padLeft(2, '0')}";
        return Row(
          crossAxisAlignment: CrossAxisAlignment.baseline,
          textBaseline: TextBaseline.alphabetic,
          children: [
            Text(
              timeStr,
              style: const TextStyle(fontSize: 35, color: Color(0xFFCCCCCC)),
            ),
            Text(
              " $ampm",
              style: const TextStyle(fontSize: 25, color: Color(0xFFCCCCCC)),
            ),
          ],
        );
      },
    );
  }
}

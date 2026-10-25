/// Generated file. Do not edit.
///
/// Original: lib/i18n
/// To regenerate, run: `dart run slang`
///
/// Locales: 2
/// Strings: 56 (28 per locale)
///
/// Built on 2026-10-25 at 00:44 UTC

// coverage:ignore-file
// ignore_for_file: type=lint

import 'package:flutter/widgets.dart';
import 'package:slang/builder/model/node.dart';
import 'package:slang_flutter/slang_flutter.dart';
export 'package:slang_flutter/slang_flutter.dart';

const AppLocale _baseLocale = AppLocale.en;

/// Supported locales, see extension methods below.
///
/// Usage:
/// - LocaleSettings.setLocale(AppLocale.en) // set locale
/// - Locale locale = AppLocale.en.flutterLocale // get flutter locale from enum
/// - if (LocaleSettings.currentLocale == AppLocale.en) // locale check
enum AppLocale with BaseAppLocale<AppLocale, Translations> {
	en(languageCode: 'en', build: Translations.build),
	zh(languageCode: 'zh', build: _StringsZh.build);

	const AppLocale({required this.languageCode, this.scriptCode, this.countryCode, required this.build}); // ignore: unused_element

	@override final String languageCode;
	@override final String? scriptCode;
	@override final String? countryCode;
	@override final TranslationBuilder<AppLocale, Translations> build;

	/// Gets current instance managed by [LocaleSettings].
	Translations get translations => LocaleSettings.instance.translationMap[this]!;
}

/// Method A: Simple
///
/// No rebuild after locale change.
/// Translation happens during initialization of the widget (call of t).
/// Configurable via 'translate_var'.
///
/// Usage:
/// String a = t.someKey.anotherKey;
/// String b = t['someKey.anotherKey']; // Only for edge cases!
Translations get t => LocaleSettings.instance.currentTranslations;

/// Method B: Advanced
///
/// All widgets using this method will trigger a rebuild when locale changes.
/// Use this if you have e.g. a settings page where the user can select the locale during runtime.
///
/// Step 1:
/// wrap your App with
/// TranslationProvider(
/// 	child: MyApp()
/// );
///
/// Step 2:
/// final t = Translations.of(context); // Get t variable.
/// String a = t.someKey.anotherKey; // Use t variable.
/// String b = t['someKey.anotherKey']; // Only for edge cases!
class TranslationProvider extends BaseTranslationProvider<AppLocale, Translations> {
	TranslationProvider({required super.child}) : super(settings: LocaleSettings.instance);

	static InheritedLocaleData<AppLocale, Translations> of(BuildContext context) => InheritedLocaleData.of<AppLocale, Translations>(context);
}

/// Method B shorthand via [BuildContext] extension method.
/// Configurable via 'translate_var'.
///
/// Usage (e.g. in a widget's build method):
/// context.t.someKey.anotherKey
extension BuildContextTranslationsExtension on BuildContext {
	Translations get t => TranslationProvider.of(this).translations;
}

/// Manages all translation instances and the current locale
class LocaleSettings extends BaseFlutterLocaleSettings<AppLocale, Translations> {
	LocaleSettings._() : super(utils: AppLocaleUtils.instance);

	static final instance = LocaleSettings._();

	// static aliases (checkout base methods for documentation)
	static AppLocale get currentLocale => instance.currentLocale;
	static Stream<AppLocale> getLocaleStream() => instance.getLocaleStream();
	static AppLocale setLocale(AppLocale locale, {bool? listenToDeviceLocale = false}) => instance.setLocale(locale, listenToDeviceLocale: listenToDeviceLocale);
	static AppLocale setLocaleRaw(String rawLocale, {bool? listenToDeviceLocale = false}) => instance.setLocaleRaw(rawLocale, listenToDeviceLocale: listenToDeviceLocale);
	static AppLocale useDeviceLocale() => instance.useDeviceLocale();
	@Deprecated('Use [AppLocaleUtils.supportedLocales]') static List<Locale> get supportedLocales => instance.supportedLocales;
	@Deprecated('Use [AppLocaleUtils.supportedLocalesRaw]') static List<String> get supportedLocalesRaw => instance.supportedLocalesRaw;
	static void setPluralResolver({String? language, AppLocale? locale, PluralResolver? cardinalResolver, PluralResolver? ordinalResolver}) => instance.setPluralResolver(
		language: language,
		locale: locale,
		cardinalResolver: cardinalResolver,
		ordinalResolver: ordinalResolver,
	);
}

/// Provides utility functions without any side effects.
class AppLocaleUtils extends BaseAppLocaleUtils<AppLocale, Translations> {
	AppLocaleUtils._() : super(baseLocale: _baseLocale, locales: AppLocale.values);

	static final instance = AppLocaleUtils._();

	// static aliases (checkout base methods for documentation)
	static AppLocale parse(String rawLocale) => instance.parse(rawLocale);
	static AppLocale parseLocaleParts({required String languageCode, String? scriptCode, String? countryCode}) => instance.parseLocaleParts(languageCode: languageCode, scriptCode: scriptCode, countryCode: countryCode);
	static AppLocale findDeviceLocale() => instance.findDeviceLocale();
	static List<Locale> get supportedLocales => instance.supportedLocales;
	static List<String> get supportedLocalesRaw => instance.supportedLocalesRaw;
}

// translations

// Path: <root>
class Translations implements BaseTranslations<AppLocale, Translations> {
	/// Returns the current translations of the given [context].
	///
	/// Usage:
	/// final t = Translations.of(context);
	static Translations of(BuildContext context) => InheritedLocaleData.of<AppLocale, Translations>(context).translations;

	/// You can call this constructor and build your own translation instance of this locale.
	/// Constructing via the enum [AppLocale.build] is preferred.
	Translations.build({Map<String, Node>? overrides, PluralResolver? cardinalResolver, PluralResolver? ordinalResolver})
		: assert(overrides == null, 'Set "translation_overrides: true" in order to enable this feature.'),
		  $meta = TranslationMetadata(
		    locale: AppLocale.en,
		    overrides: overrides ?? {},
		    cardinalResolver: cardinalResolver,
		    ordinalResolver: ordinalResolver,
		  ) {
		$meta.setFlatMapFunction(_flatMapFunction);
	}

	/// Metadata for the translations of <en>.
	@override final TranslationMetadata<AppLocale, Translations> $meta;

	/// Access flat map
	dynamic operator[](String key) => $meta.getTranslation(key);

	late final Translations _root = this; // ignore: unused_field

	// Translations
	late final _StringsDashboardEn dashboard = _StringsDashboardEn._(_root);
	late final _StringsScannerEn scanner = _StringsScannerEn._(_root);
}

// Path: dashboard
class _StringsDashboardEn {
	_StringsDashboardEn._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations
	String get power => 'Power';
	String get alarm => 'Alarm';
	String get lights => 'Lights';
	String get sport => 'Sport';
	String get cruise => 'Cruise';
	String get park => 'Park';
	String get brake => 'Brake';
	String get horn => 'Horn';
	String get entry => 'Entry';
	String get cruise_up => 'Cruise\nUp';
	String get cruise_down => 'Cruise\nDown';
	late final _StringsDashboardUnitsEn units = _StringsDashboardUnitsEn._(_root);
	late final _StringsDashboardIndicatorsEn indicators = _StringsDashboardIndicatorsEn._(_root);
}

// Path: scanner
class _StringsScannerEn {
	_StringsScannerEn._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations
	String get title => 'Log';
	String get scanning => 'Scanning...';
	String get connecting => 'Connecting...';
	String get disconnected => 'Disconnected';
	String get connect => 'Connect';
	String get disconnect => 'Disconnect';
	String get unknown => 'Unknown Device';
	String get nearby => 'Nearby Devices';
	String get log => 'Log';
	String get clear => 'clear';
}

// Path: dashboard.units
class _StringsDashboardUnitsEn {
	_StringsDashboardUnitsEn._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations
	String get kmh => 'km/h';
	String get km => 'km';
	String get celsius => '°C';
	String get mv => 'mv';
	String get watts => 'w';
}

// Path: dashboard.indicators
class _StringsDashboardIndicatorsEn {
	_StringsDashboardIndicatorsEn._(this._root);

	final Translations _root; // ignore: unused_field

	// Translations
	String get on => 'ON';
	String get off => 'OFF';
}

// Path: <root>
class _StringsZh implements Translations {
	/// You can call this constructor and build your own translation instance of this locale.
	/// Constructing via the enum [AppLocale.build] is preferred.
	_StringsZh.build({Map<String, Node>? overrides, PluralResolver? cardinalResolver, PluralResolver? ordinalResolver})
		: assert(overrides == null, 'Set "translation_overrides: true" in order to enable this feature.'),
		  $meta = TranslationMetadata(
		    locale: AppLocale.zh,
		    overrides: overrides ?? {},
		    cardinalResolver: cardinalResolver,
		    ordinalResolver: ordinalResolver,
		  ) {
		$meta.setFlatMapFunction(_flatMapFunction);
	}

	/// Metadata for the translations of <zh>.
	@override final TranslationMetadata<AppLocale, Translations> $meta;

	/// Access flat map
	@override dynamic operator[](String key) => $meta.getTranslation(key);

	@override late final _StringsZh _root = this; // ignore: unused_field

	// Translations
	@override late final _StringsDashboardZh dashboard = _StringsDashboardZh._(_root);
	@override late final _StringsScannerZh scanner = _StringsScannerZh._(_root);
}

// Path: dashboard
class _StringsDashboardZh implements _StringsDashboardEn {
	_StringsDashboardZh._(this._root);

	@override final _StringsZh _root; // ignore: unused_field

	// Translations
	@override String get power => '电源';
	@override String get alarm => '报警器';
	@override String get lights => '车灯';
	@override String get sport => '运动模式';
	@override String get cruise => '定速巡航';
	@override String get park => '驻车';
	@override String get brake => '刹车';
	@override String get horn => '喇叭';
	@override String get entry => '门禁';
	@override String get cruise_up => '巡航\n加速';
	@override String get cruise_down => '巡航\n减速';
	@override late final _StringsDashboardUnitsZh units = _StringsDashboardUnitsZh._(_root);
	@override late final _StringsDashboardIndicatorsZh indicators = _StringsDashboardIndicatorsZh._(_root);
}

// Path: scanner
class _StringsScannerZh implements _StringsScannerEn {
	_StringsScannerZh._(this._root);

	@override final _StringsZh _root; // ignore: unused_field

	// Translations
	@override String get title => '日志';
	@override String get scanning => '扫描中...';
	@override String get connecting => '连接中...';
	@override String get disconnected => '已断开';
	@override String get connect => '连接';
	@override String get disconnect => '断开';
	@override String get unknown => '未知设备';
	@override String get nearby => '附近设备';
	@override String get log => '日志';
	@override String get clear => '清除';
}

// Path: dashboard.units
class _StringsDashboardUnitsZh implements _StringsDashboardUnitsEn {
	_StringsDashboardUnitsZh._(this._root);

	@override final _StringsZh _root; // ignore: unused_field

	// Translations
	@override String get kmh => 'km/h';
	@override String get km => 'km';
	@override String get celsius => '°C';
	@override String get mv => 'mv';
	@override String get watts => 'w';
}

// Path: dashboard.indicators
class _StringsDashboardIndicatorsZh implements _StringsDashboardIndicatorsEn {
	_StringsDashboardIndicatorsZh._(this._root);

	@override final _StringsZh _root; // ignore: unused_field

	// Translations
	@override String get on => '开';
	@override String get off => '关';
}

/// Flat map(s) containing all translations.
/// Only for edge cases! For simple maps, use the map function of this library.

extension on Translations {
	dynamic _flatMapFunction(String path) {
		switch (path) {
			case 'dashboard.power': return 'Power';
			case 'dashboard.alarm': return 'Alarm';
			case 'dashboard.lights': return 'Lights';
			case 'dashboard.sport': return 'Sport';
			case 'dashboard.cruise': return 'Cruise';
			case 'dashboard.park': return 'Park';
			case 'dashboard.brake': return 'Brake';
			case 'dashboard.horn': return 'Horn';
			case 'dashboard.entry': return 'Entry';
			case 'dashboard.cruise_up': return 'Cruise\nUp';
			case 'dashboard.cruise_down': return 'Cruise\nDown';
			case 'dashboard.units.kmh': return 'km/h';
			case 'dashboard.units.km': return 'km';
			case 'dashboard.units.celsius': return '°C';
			case 'dashboard.units.mv': return 'mv';
			case 'dashboard.units.watts': return 'w';
			case 'dashboard.indicators.on': return 'ON';
			case 'dashboard.indicators.off': return 'OFF';
			case 'scanner.title': return 'Log';
			case 'scanner.scanning': return 'Scanning...';
			case 'scanner.connecting': return 'Connecting...';
			case 'scanner.disconnected': return 'Disconnected';
			case 'scanner.connect': return 'Connect';
			case 'scanner.disconnect': return 'Disconnect';
			case 'scanner.unknown': return 'Unknown Device';
			case 'scanner.nearby': return 'Nearby Devices';
			case 'scanner.log': return 'Log';
			case 'scanner.clear': return 'clear';
			default: return null;
		}
	}
}

extension on _StringsZh {
	dynamic _flatMapFunction(String path) {
		switch (path) {
			case 'dashboard.power': return '电源';
			case 'dashboard.alarm': return '报警器';
			case 'dashboard.lights': return '车灯';
			case 'dashboard.sport': return '运动模式';
			case 'dashboard.cruise': return '定速巡航';
			case 'dashboard.park': return '驻车';
			case 'dashboard.brake': return '刹车';
			case 'dashboard.horn': return '喇叭';
			case 'dashboard.entry': return '门禁';
			case 'dashboard.cruise_up': return '巡航\n加速';
			case 'dashboard.cruise_down': return '巡航\n减速';
			case 'dashboard.units.kmh': return 'km/h';
			case 'dashboard.units.km': return 'km';
			case 'dashboard.units.celsius': return '°C';
			case 'dashboard.units.mv': return 'mv';
			case 'dashboard.units.watts': return 'w';
			case 'dashboard.indicators.on': return '开';
			case 'dashboard.indicators.off': return '关';
			case 'scanner.title': return '日志';
			case 'scanner.scanning': return '扫描中...';
			case 'scanner.connecting': return '连接中...';
			case 'scanner.disconnected': return '已断开';
			case 'scanner.connect': return '连接';
			case 'scanner.disconnect': return '断开';
			case 'scanner.unknown': return '未知设备';
			case 'scanner.nearby': return '附近设备';
			case 'scanner.log': return '日志';
			case 'scanner.clear': return '清除';
			default: return null;
		}
	}
}

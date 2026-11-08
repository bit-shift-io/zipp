#!/bin/bash

flutter_rust_bridge_codegen generate
dart run slang
flutter doctor
adb devices

# build
flutter build apk --release

# copy to phone
flutter install

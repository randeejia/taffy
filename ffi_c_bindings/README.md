# 项目简介

本库基于github开源库[taffy](https://github.com/DioxusLabs/taffy)封装的FFI，当前仅有`c bindings`，以供Android/ios/HarmonyOS使用

**注意**: 本工程全部由AI Agent撰写


## 构建产物

### 编译Android
1. 安装 Rust 的 Android 目标
```bash
rustup target add aarch64-linux-android
```
2. 构建产物`.so`
```bash
cd ffi_c_bindings
# Debug版
cargo build --target aarch64-linux-android
# Release版
cargo build --release --target aarch64-linux-android
```

### 编译鸿蒙
1. 安装 Rust 的 鸿蒙 目标，arm64，x86_64
```bash
# Target Arm64
rustup target add aarch64-unknown-linux-ohos
# Target x86_64
rustup target add x86_64-unknown-linux-ohos
```
2. 构建产物`.so`
```bash
cd ffi_c_bindings

# Target Arm64 Debug版
cargo build --target aarch64-unknown-linux-ohos
# Target Arm64 Release版
cargo build --release --target aarch64-unknown-linux-ohos

# Target x86_64 Debug版
cargo build --target x86_64-unknown-linux-ohos
# Target x86_64 Release版
cargo build --release --target x86_64-unknown-linux-ohos
```
另：如需减小产物体积，可用 OHOS SDK 的`llvm-strip` 去除调试符号：
```bash
/Applications/DevEco-Studio.app/Contents/sdk/default/openharmony/native/llvm/bin/llvm-strip \
target/aarch64-unknown-linux-ohos/release/libtaffy.so
```


## 构建目标配置

在`./ffi_c/.cargo/config.toml`文件下配置linker和target

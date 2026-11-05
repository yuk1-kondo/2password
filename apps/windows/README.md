# 2Password Windows版 — 設計メモ（Mac上ではビルド不可）

## 方針
- 共有Rustコアはそのまま利用。FFI経路は2案:
  1. **csbindgen**（推奨）: `twopassword-ffi`をC ABIで公開→C# P/Invoke自動生成。
     crates/ffiに`#[no_mangle] extern "C"`関数を追加 or csbindgen属性で生成
  2. UniFFI C#バックエンドは実験的なので避ける
- UI: **WinUI 3 (C# + WinAppSDK)** — Windows Hello (UserConsentVerifier) +
  DPAPI/TPMでAUK wrap（仕様§8.2、Android/Appleと同じ3層）

## Windows側で必要な作業
1. `cargo build --release -p twopassword-ffi` → `twopassword_ffi.dll`
2. csbindgenで`NativeMethods.g.cs`生成（ffi/srcに`csbindgen`メタ or 生成スクリプト）
3. `dotnet new winui`でアプリひな形→VaultStore相当をC#で実装
4. Secret Key保管: `Windows.Security.Credentials.PasswordVault` or DPAPIファイル
5. Windows Hello: `UserConsentVerifier.RequestVerificationAsync`ゲートでAUK unwrap

## ビルド確認はWindows機で行うこと（このMacではコンパイル不可）

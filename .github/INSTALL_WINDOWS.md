# تثبيت Ferrox على Windows

للتثبيت الأسهل، حمّل `Ferrox-Setup-...-x86_64.exe` من صفحة [الإصدارات](https://github.com/Ahmaddragon11/ferrox/releases) وشغّله. سيثبّت WebView2 عند الحاجة، وينشئ اختصاراً بأيقونة Ferrox على سطح المكتب وفي قائمة ابدأ، ويمكنك تشغيل المتصفح مباشرةً من شاشة الإنهاء.

للتحقق من المثبّت قبل تشغيله، نفّذ:

```powershell
CertUtil -hashfile Ferrox-Setup-<VERSION>-x86_64.exe SHA256
```

قارن الناتج مع `Ferrox-Setup-<VERSION>-x86_64.exe.sha256` المرفق في صفحة الإصدار.

## النسخة المحمولة:

1. **حمّل** أحدث نسخة `ferrox-windows-x86_64-...zip` من صفحة [الإصدارات](https://github.com/Ahmaddragon11/ferrox/releases).
2. **تحقق** من صحة الملف:
   ```powershell
   CertUtil -hashfile ferrox-windows-x86_64-<VERSION>.zip SHA256
   ```
   قارن القيمة مع الملف `ferrox-windows-x86_64-...zip.sha256`.
3. **استخرج** الملفات في أي مجلد (مثلاً `C:\Apps\Ferrox`).
4. ثبّت [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/) إذا لم يكن مثبتاً.
5. انقر مرتين على `ferrox.exe` لبدء التصفح. يمكن إنشاء اختصار يدوياً من قائمة النقر بالزر الأيمن.

## حلول المشاكل الشائعة:

- **خطأ "Missing MSVCP140.dll"**: حمل [Visual C++ Redistributable](https://aka.ms/vs/17/release/vc_redist.x64.exe)
- **شاشة سوداء أو صفحات لا تفتح**: يحتاج Ferrox إلى [Microsoft Edge WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/). مثبّت Ferrox يثبّته تلقائياً؛ أما النسخة المحمولة فتحتاج إلى تثبيته يدوياً عند عدم وجوده.
- **فشل فتح الصفحة بعد التثبيت**: أعد تشغيل Ferrox بعد تثبيت WebView2 Runtime. ستظهر رسالة داخل المتصفح إذا تعذّر إنشاء محرك عرض الصفحات.

## تحديث النسخة:

عند إصدار نسخة جديدة، احمل النسخة الجديدة واستبدل الملف القديم في نفس المجلد.


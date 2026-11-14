# تثبيت Ferrox على Windows

Ferrox هو متصفح **قابل للنقل (Portable)** - لا يحتاج إلى مثبّت!

## الطريقة الموصى بها (Portable):

1. **حمل** أحدث نسخة من `ferrox-windows-x86_64.zip` من صفحة [الإصدارات](https://github.com/Ahmaddragon11/ferrox/releases)
2. **تحقق** من صحة الملف:
   ```powershell
   CertUtil -hashfile ferrox-windows-x86_64.zip SHA256
   ```
   قارن القيمة مع الملف `ferrox-windows-x86_64.zip.sha256`
3. **استخرج** الملفات في أي مجلد (مثلاً `C:\Apps\Ferrox`)
4. **انقر مرتين** على `ferrox.exe` لبدء التصفح!

## التثبيت الدائم (اختياري):

```powershell
# إنشاء مجلد التثبيت
New-Item -ItemType Directory -Force -Path "C:\Program Files\Ferrox"
# نقل الملف
Move-Item -Path ".\ferrox.exe" -Destination "C:\Program Files\Ferrox\"
# إضافة اختصار إلى سطح المكتب
$WshShell = New-Object -ComObject WScript.Shell
$Shortcut = $WshShell.CreateShortcut("$HOME\Desktop\Ferrox.lnk")
$Shortcut.TargetPath = "C:\Program Files\Ferrox\ferrox.exe"
$Shortcut.Save()
```

## حلول المشاكل الشائعة:

- **خطأ "Missing MSVCP140.dll"**: حمل [Visual C++ Redistributable](https://aka.ms/vs/17/release/vc_redist.x64.exe)
- **فشل فتح الصفحة**: تأكد من تثبيت WebKit - عادة ما يكون موجوداً مع Windows 10/11

## تحديث النسخة:

عند إصدار نسخة جديدة، احمل النسخة الجديدة واستبدل الملف القديم في نفس المجلد.


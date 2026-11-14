# Ferrox Windows Installer (NSIS)

هذا المجلد يحتوي على سكربت NSIS لبناء مثبّت Windows الخاص بـ Ferrox.

## الاستخدام

سكربت NSIS (`.nsi`) لا يُنفَّذ مباشرةً - بل يتم تعميته وتعبئته بواسطة GitHub Actions:

```
.github/workflows/release.yml
  ├─ build-windows job
  │   └─ بناء ferrox.exe وملف Windows المحمول ورفعهما
  └─ build-linux job
      ├─ تثبيت NSIS من مستودع Ubuntu الرسمي
      ├─ تنزيل ملف Windows التنفيذي وتجهيز قالب المثبّت
      └─ بناء مثبّت Windows والتحقق من SHA-256
```

يُبنى ملف التثبيت على Linux باستخدام NSIS متعدد المنصات، لتجنب الاعتماد على مُثبّت NSIS من SourceForge في Windows runner.

## المتغيرات المُستبدلة

| المتغير | القيمة |
|----------|--------|
| `@VERSION@` | نسخة Ferrox (مثال: `0.2.0` أو `0.2.0-abc1234`) |
| `@INSTALLER_NAME@` | اسم الملف النهائي (مثال: `Ferrox-Setup-0.2.0-x86_64.exe`) |
| `@EXE_PATH@` | مسار ملف `ferrox.exe` المبنى |
| `@WEBVIEW2_PATH@` | مسار WebView2 Runtime bootstrapper المضمّن |
| `@ICON_PATH@` | مسار أيقونة Ferrox بصيغة ICO |

## متطلبات البناء

- **makensis** (NSIS 3.x) — يُثبّت تلقائياً عبر حزم Ubuntu في workflow الإصدار.
- يجب توفير قيم المتغيرات المذكورة أعلاه قبل البناء اليدوي.
- يفشل إصدار Windows إذا تعذّر بناء المثبّت بدلاً من نشر ملفات ناقصة.

## مميزات المثبّت

1. تثبيت في `C:\Program Files\Ferrox\`
2. تثبيت WebView2 Runtime عند عدم وجوده
3. أيقونة Ferrox في اختصار سطح المكتب وقائمة ابدأ والمثبّت نفسه
4. خيار تشغيل Ferrox فور انتهاء التثبيت
5. تسجيل صحيح ضمن Windows (Add/Remove Programs)
6. إلغاء تثبيت ينظف الاختصارات والملفات دون حذف بيانات المستخدم

## أدوات مساعدة

```bash
# على Ubuntu، بعد استبدال متغيرات القالب (يتطلب NSIS)
makensis -V1 .github/installer/ferrox-installer.nsi
```

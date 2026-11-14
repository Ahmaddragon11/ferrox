# Ferrox Windows Installer (NSIS)

هذا المجلد يحتوي على سكربت NSIS لبناء المثبّت الآلي لـ Ferrox على Windows.

## الاستخدام

سكربت NSIS (`.nsi`) لا يُنفَّذ مباشرةً - بل يتم تعميته وتعبئته بواسطة GitHub Actions:

```
.github/workflows/release.yml
  └─ build-windows job
      ├─ Prepare NSIS installer   (يقرأ ferrox.nsi، يحل المتغيرات، يكتب ferrox-installer.nsi)
      └─ Build NSIS installer     (يشغل makensis إذا كان متاحاً)
```

## المتغيرات المُستبدلة

| المتغير | القيمة |
|----------|--------|
| `@VERSION@` | نسخة Ferrox (مثال: `0.2.0` أو `0.2.0-abc1234`) |
| `@INSTALLER_NAME@` | اسم الملف النهائي (مثال: `Ferrox-Setup-0.2.0-x86_64.exe`) |
| `@EXE_PATH@` | مسار ملف `ferrox.exe` المبنى |
| `@EXE_HASH@` | SHA256 لملف `ferrox.exe` |

## متطلبات البناء

- **makensis** (NSIS 3.x) متاح على runner — يُستخدم لإنشاء المثبّت الآلي
- إذا لم يكن makensis متاحاً: يتم تجاوز خطوة البناء وينتج الـ workflow حزمة **portable** فقط (`ferrox-windows-x86_64-*.zip`)

## مميزات المثبّت

1. تثبيت في `C:\Program Files\Ferrox\`
2. اختصار على سطح المكتب
3. اختصار في قائمة البدء
4. سجل Windows (Add/Remove Programs)
5. إلغاء تثبيت آلي
6. ترقية تلقائية عند اكتشاف تثبيت سابق

## أدوات مساعدة

```powershell
# بناء المثبّت يدوياً (يتطلب NSIS)
makensis /V1 .github\installer\ferrox-installer.nsi
```

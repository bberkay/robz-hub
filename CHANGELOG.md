# Değişiklikler

Her sürümün notları burada `## X.Y.Z` başlığı altında durur; sürüm iş akışı (.github/workflows/release.yml) GitHub
sürüm notunu bu bölümden alır.

## 0.1.4

<!-- tr -->
- **Dışarıda yapılan değişiklikler birkaç saniyede görünür.** Bir uygulamayı kendi içindeki "Güncelle" düğmesiyle güncellersen, Robz Hub dışında kurarsan ya da Windows Ayarlar'dan kaldırırsan Robz Hub bunu birkaç saniye içinde görür; penceresi kapalıyken ya da tepsideyken de. Sürüm, "Güncelleme var" sayacı ve tepsi menüsü kendiliğinden güncellenir.

<!-- en -->
<details><summary>English</summary>

- **Changes made elsewhere show up within seconds.** If you update an app with its own "Update" button, install it outside Robz Hub or uninstall it from Windows Settings, Robz Hub notices within a few seconds, even while its window is closed or it's in the tray. The version, the update counter and the tray menu update on their own.

</details>
<!-- end -->

## 0.1.3

<!-- tr -->
- **İndirme sayıları.** Her uygulamanın GitHub'daki toplam indirme sayısı uygulama listesinde ve uygulama sayfasında, her sürümün indirme sayısı sürüm notlarında görünür.
- **Bayraklı dil seçimi.** Kenar çubuğundaki TR / EN düğmelerinin yerine Türk ve İngiliz bayraklı bir seçim kutusu geldi; seçtiğin an dil değişir. Ayarlar'da "Windows dili" seçeneği de bu kutuda.

<!-- en -->
<details><summary>English</summary>

- **Download counts.** Each app's total downloads on GitHub show in the app list and on the app page, and each release's download count shows in the release notes.
- **Flag language picker.** The TR / EN buttons in the sidebar are replaced by a picker with the Turkish and British flags; the language changes as soon as you choose. In Settings, the "Windows language" option is in the same picker.

</details>
<!-- end -->

## 0.1.2

<!-- tr -->
- Robz Hub kendini güncelledikten sonra, güncelleme için indirdiği kurulum dosyasını siler; indirilenler klasöründe yer kaplamaz.
- Sırada bekleyen bir kurulumun ilerleme çubuğu artık boş durur; çubuk yalnız indirme, kapatma, kurma ve kaldırma sırasında hareket eder.

<!-- en -->
<details><summary>English</summary>

- After updating itself, Robz Hub deletes the installer it downloaded for the update, so it no longer takes up space in the downloads folder.
- A queued install's progress bar now stays empty; the bar only moves while downloading, closing, installing or uninstalling.

</details>
<!-- end -->

## 0.1.1

<!-- tr -->
- **Yalnız kurulu kopya kapatılır.** Bir uygulamanın açık olup olmadığı artık exe adıyla değil, kurulu olduğu klasörle anlaşılır. Aynı adla başka bir yerden çalışan bir kopya (ör. geliştirme klasöründen açılmış bir derleme) "Çalışıyor" sayılmaz; kurulum, güncelleme ve "Kapat" ona dokunmaz.
- Paket kutusunda bir uygulama eksik, diğerinde güncelleme varken düğme "Kur ve güncelle" yazar.
- Robz Hub'ın kendi sürüm notları seçili dilde gösterilir.
- Yeni turuncu logo ve ana düğmeler.

<!-- en -->
<details><summary>English</summary>

- **Only the installed copy is closed.** Whether an app is running is now decided by the folder it's installed in, not by its exe name. A copy with the same name running from elsewhere (for example a build in a development folder) doesn't count as "Running", and installing, updating or "Close" leave it alone.
- When one app in a bundle is missing and the other has an update, the button reads "Install and update".
- Robz Hub's own release notes show in the selected language.
- New orange logo and primary buttons.

</details>
<!-- end -->

## 0.1.0

<!-- tr -->
İlk sürüm.

- **Tek yerden kur, güncelle, kaldır.** Robz Play ve Robz Vision tek tıkla kurulur; yeni sürüm çıkınca "Güncelle" düğmesi belirir. Kurulum dosyası GitHub'dan iner ve SHA-256 özetiyle doğrulanır.
- **Başlat ve kapat.** Kurulu uygulamalar buradan açılır ve kapatılır; çalışanlar yeşil noktayla görünür.
- **GTA VI paketi.** "İkisini de kur" ile Robz Play (60 FPS) ve Robz Vision (Türkçe altyazı) birlikte kurulur, "İkisini de başlat" ile birlikte açılır.
- **Sürüm notları.** Her uygulamanın bütün sürümleri; kurulu sürümden sonra gelenler "Yeni" etiketiyle.
- **Türkçe ve İngilizce.** Kenar çubuğundaki TR / EN ile değişir.
- **Tepside çalışma, Windows açılışında başlatma ve yeni sürüm bildirimi** (Ayarlar'dan).

<!-- en -->
<details><summary>English</summary>

First release.

- **Install, update and uninstall from one place.** Robz Play and Robz Vision install with one click; an "Update" button appears when a new version is out. Installers download from GitHub and are verified against their SHA-256 digest.
- **Launch and close.** Open and close installed apps from here; running ones show a green dot.
- **GTA VI bundle.** "Install both" sets up Robz Play (60 FPS) and Robz Vision (Turkish subtitles) together; "Launch both" opens them together.
- **Release notes.** Every release of every app; the ones newer than your installed version are marked "New".
- **Turkish and English.** Switch with TR / EN in the sidebar.
- **Tray mode, start with Windows and update notifications** (in Settings).

</details>
<!-- end -->

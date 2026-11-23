# Değişiklikler

Her sürümün notları burada `## X.Y.Z` başlığı altında durur; sürüm iş akışı (.github/workflows/release.yml) GitHub
sürüm notunu bu bölümden alır.

## 0.1.6

<!-- tr -->
- **Linux desteği.** Robz Hub artık Linux'ta da çalışır: .deb (Debian, Ubuntu, Mint), .rpm (Fedora, openSUSE) ve AppImage. Linux paketi olan uygulamalar paket yöneticisiyle kurulur, güncellenir ve kaldırılır; bunun için sistemin parola penceresi açılır. Linux sürümü olmayan uygulamalarda "Linux'ta desteklenmiyor" yazar. Robz Hub kendini Linux'ta da günceller.
- **Yeni uygulamaları kendisi bulur.** GitHub'da robz- ile başlayan yeni bir depo kurulum dosyası yayınlayınca, kataloğa eklenmeyi beklemeden Robz Hub'da görünür ve bildirim gelir. Denetim saatte en çok bir kez, tek istekle yapılır.
- **Robz DPI eklendi.** Yalnız seçtiğin sitelerde çalışan açık kaynak DPI aracı; Windows'ta ve Linux'ta kurulur.
- **İndirme ve yıldız sayıları.** Uygulama sayfasında GitHub'daki toplam indirme ve yıldız sayısı görünür; ana sayfadaki kartlarda indirme sayısı beyaz, yıldız sarı.
- Simgesi yüklenemeyen uygulamada adının baş harfleri görünür.

<!-- en -->
<details><summary>English</summary>

- **Linux support.** Robz Hub now runs on Linux too: .deb (Debian, Ubuntu, Mint), .rpm (Fedora, openSUSE) and AppImage. Apps with a Linux package install, update and uninstall through your package manager, which opens your system's password prompt. Apps without a Linux version say "Not available on Linux". Robz Hub updates itself on Linux too.
- **Finds new apps on its own.** When a new GitHub repository starting with robz- publishes an installer, it shows up in Robz Hub without waiting for the catalog, and you get a notification. It checks at most once an hour, with a single request.
- **Robz DPI added.** An open-source DPI tool that works only on the sites you choose; installs on Windows and Linux.
- **Downloads and stars.** The app page shows the total downloads and stars on GitHub; on the home page cards, downloads are white and stars yellow.
- An app whose icon can't load shows the initials of its name.

</details>
<!-- end -->

## 0.1.5

<!-- tr -->
- **Yeni ana sayfa.** Üstte dönen bir tanıtım alanı var: GTA VI paketi ve duyurular. Tek düğmeyle iki uygulamayı birden kurar, günceller ya da başlatırsın. Altında kapak görselli uygulama kartları, bütün uygulamaların son sürümleri ve Topluluk bölümü yer alır.
- **İlk duyurular Discord'da.** Yeni sürümler ve uygulamalar her zaman önce Discord sunucusunda duyurulur. Topluluk bölümünde Discord en üstte; "Sıradaki Robz uygulaması" kartı da Discord'a götürür. Canlı yayınlar Kick'te.
- **Gerçek Discord ve Kick logoları** kenar çubuğunda, Ayarlar'da ve ana sayfada.
- **Atlanan sürümlerin notları.** Robz Hub'ın birkaç sürümünü atladıysan, güncelleme kartı aradaki bütün sürümlerin notlarını gösterir; en yenisi üstte.
- **Düzeltme:** GitHub'da yeni bir sürüm çıktığında "Güncelleme var", son sürüm ve indirme sayısı hemen yenilenir. 0.1.4'te bunlar, uygulamanın kurulumu değişene kadar eski kalabiliyordu.

0.1.2 ile 0.1.4 arasında gelenler de bu sürümde:
- Bayraklı dil seçimi.
- GitHub indirme sayıları.
- Uygulamanın kendi içinden yapılan güncellemeler birkaç saniyede görünür.
- Kendini güncelledikten sonra indirdiği kurulum dosyasını siler.

<!-- en -->
<details><summary>English</summary>

- **New home page.** At the top, a rotating promo area shows the GTA VI bundle and announcements. One button installs, updates or launches both apps at once. Below it are app cards with cover images, the latest releases of every app and a Community section.
- **Announced first on Discord.** New releases and apps are always announced on the Discord server first. Discord is at the top of the Community section, and "The next Robz app" card links to Discord. Live streams are on Kick.
- **Real Discord and Kick logos** in the sidebar, Settings and the home page.
- **Notes for skipped versions.** If you skipped a few Robz Hub versions, the update card shows the notes of every version in between, newest first.
- **Fix:** when a new release appears on GitHub, "Update available", the latest version and the download count refresh right away. In 0.1.4 they could stay stale until the app's installation changed.

Also in this release, from 0.1.2 to 0.1.4:
- A language picker with flags.
- GitHub download counts.
- Updates made inside an app show up within seconds.
- Robz Hub deletes the installer it downloaded after updating itself.

</details>
<!-- end -->

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

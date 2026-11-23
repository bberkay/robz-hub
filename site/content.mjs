// Web sitesinin metinleri. Uygulamaların adları, açıklamaları ve bağlantıları buradan değil, ../catalog.json'dan gelir.
export default {
  tr: {
    lang: 'tr',
    locale: 'tr_TR',
    title: 'Robz Hub: bütün Robz uygulamaları tek yerde',
    description:
      'Robz Play ve Robz Vision gibi Robz açık kaynak uygulamalarını tek tıkla kur, güncelle ve başlat. Rust ile yazıldı, 2 MB, ücretsiz.',
    skip: 'İçeriğe geç',
    nav: { apps: 'Uygulamalar', usage: 'Kullanım', faq: 'Sık sorulanlar', github: 'GitHub' },
    releasePill: 'Bu sürümde neler var',
    heroTitle: 'Bütün Robz uygulamaları tek yerde.',
    heroLead:
      "Robz Hub, Robz'un açık kaynak uygulamalarını tek tıkla kurar, güncel tutar ve başlatır. GTA VI'yı Robz Play ile 60 FPS oyna, altyazısını Robz Vision ile Türkçe oku: ikisi de buradan.",
    download: 'Windows için indir',
    downloadLinux: 'Linux için indir',
    linux: 'Linux için:',
    otherFormats: 'Diğer biçimler:',
    portableLabel: 'Windows taşınabilir (kurulumsuz)',
    debLabel: '.deb (Debian, Ubuntu)',
    rpmLabel: '.rpm (Fedora, openSUSE)',
    appimageLabel: 'AppImage (diğer dağıtımlar)',
    star: 'Yıldız ver',
    starTitle: "GitHub'da yıldız ver",
    facts: ['Windows ve Linux', 'Yeni uygulamaları kendisi bulur', 'Rust ile yazıldı', 'Ücretsiz ve açık kaynak'],
    shotAlt:
      'Robz Hub ana sayfası: üstte GTA VI paketi, altında Robz Play ve Robz Vision listesi; Robz Vision için 1.3.2 sürümünden 1.6.0 sürümüne güncelleme.',

    whyTitle: 'Neden Robz Hub?',
    whyLead:
      'Robz uygulamaları birlikte kullanılmak için yapılıyor. Robz Hub, onları ayrı ayrı aramak, indirmek ve güncel tutmak yerine tek bir yerden yönetmek için var.',
    diffFile: 'robz-kurulumu.diff',
    diffLabel: 'Eskiden ve Robz Hub ile kurulum karşılaştırması',
    diff: {
      del: [
        "Robz Play'in sayfasına gir, doğru kurulum dosyasını bul, indir, kur",
        "Robz Vision'ın sayfasına gir, Setup mı Portable mı diye seç, indir, kur",
        'Yeni sürüm çıktı mı? İki depoyu ayrı ayrı denetle',
      ],
      add: [
        "Robz Hub'ı bir kez kur",
        '"İkisini de kur"a bas; dosyalar iner, doğrulanır, kurulur',
        'Yeni sürüm çıkınca bildirim gelir, "Güncelle"ye bas',
      ],
    },

    appsTitle: 'İçindeki uygulamalar',
    appsLead:
      "Liste GitHub'daki katalogdan gelir: yeni bir Robz uygulaması yayınlandığında, Robz Hub'ı güncellemeden listede belirir.",
    latest: 'Son sürüm',
    appWebsite: 'Web sitesi',
    appRepo: 'Kaynak kodu',
    bundleNote:
      'Robz Hub\'da bu kutunun "İkisini de kur" düğmesi iki uygulamayı sırayla indirip kurar; "İkisini de başlat" ikisini birlikte açar.',

    usageTitle: 'Nasıl kullanılır',
    usageLead: 'İlk kurulumdan oyuna dört adım.',
    steps: [
      {
        title: "Robz Hub'ı indir ve kur",
        text: 'RobzHub-Setup.exe dosyasını çalıştır. Yönetici izni istemez, yalnız senin kullanıcı hesabına kurulur ve birkaç saniye sürer.',
        note: 'Dosya henüz imzalı olmadığı için Windows "bilgisayarınızı korudu" uyarısı gösterebilir: Ek bilgi › Yine de çalıştır.',
      },
      {
        title: 'Uygulamaları kur',
        text: 'Ana sayfadaki "İkisini de kur" ile Robz Play ve Robz Vision birlikte kurulur; tek tek kurmak için listedeki "Kur" düğmesini kullan. İndirme ilerlemesi, hızı ve kalan süre görünür; istersen iptal edebilirsin.',
        shot: '01-ana-sayfa.png',
        shotAlt: 'Ana sayfa: GTA VI paketi ve uygulama listesi',
      },
      {
        title: 'Başlat',
        text: 'Kurulu uygulamalar "Başlat" düğmesiyle açılır; çalışanların yanında yeşil nokta yanar, "Kapat" ile kapatılır. Pencere kapalıyken tepsi simgesinin menüsünden de başlatabilirsin.',
      },
      {
        title: 'Güncel kal',
        text: 'Yeni sürüm çıkınca kenar çubuğunda sayaç belirir ve Windows bildirimi gelir. Güncellemeler sayfası neyin değiştiğini gösterir; "Güncelle" gerisini halleder, ayarların korunur.',
        shot: '04-guncellemeler.png',
        shotAlt: 'Güncellemeler sayfası: Robz Vision 1.3.2 sürümünden 1.6.0 sürümüne, sürüm notlarıyla',
      },
    ],

    detailsTitle: 'Ayrıntılar',
    details: [
      ['Küçük ve hızlı', "Rust ve Windows'un kendi WebView2'siyle (Tauri) yazıldı. Kurulum dosyası yaklaşık 2 MB, açılışı anlık."],
      ['İndirilen her dosya doğrulanır', "Kurulum dosyası yalnız uygulamanın kendi GitHub sürümünden iner; SHA-256 özeti GitHub'ınkiyle tutmazsa silinir ve çalıştırılmaz."],
      ['Veri toplamaz', 'Hesap, reklam ya da kullanım istatistiği yok. Robz Hub yalnız GitHub ile konuşur ve kendini "RobzHub/sürüm" diye tanıtır.'],
      ['Sürüm notları ve indirme sayıları', 'Her uygulamanın bütün sürümleri tarihleri ve indirme sayılarıyla; kurulu sürümünden sonra gelenler "Yeni" etiketiyle işaretli.'],
      ['Arka planda bekler', 'İstersen tepside çalışır, Windows açılışında başlar ve yeni sürümü bildirir. Hepsi Ayarlar\'dan açılıp kapanır.'],
      ['Türkçe ve İngilizce', 'Arayüz Windows diline göre açılır; kenar çubuğundaki bayraklı dil seçiminden değişir.'],
    ],

    screensTitle: 'Ekranlar',
    screens: [
      ['02-uygulama.png', 'Uygulama sayfası: ne yaptığı, gerekenler, kurulum yeri ve işlemler.'],
      ['03-surum-notlari.png', 'Sürüm notları: her sürüm tarihiyle, en yenisi üstte.'],
      ['05-ayarlar.png', 'Ayarlar: dil, tepside çalışma, Windows açılışı, bildirimler.'],
    ],
    openImage: 'Görseli büyüt',

    faqTitle: 'Sık sorulanlar',
    faq: [
      [
        'Robz Hub ücretli mi?',
        'Hayır. Robz Hub ve içindeki uygulamalar ücretsiz ve açık kaynaktır. Hesap açman ya da bir şey ödemen gerekmez.',
      ],
      [
        'Uygulamaları zaten kurduysam ne olur?',
        'Robz Hub, Windows\'un "Yüklü uygulamalar" kaydına ve uygulamaların klasörlerine bakarak kurulu olanları ve sürümlerini kendisi bulur. Yeniden kurman gerekmez; yeni sürüm varsa "Güncelle" düğmesi görünür. Uygulamayı kendi içinden güncellersen Robz Hub bunu birkaç saniye içinde görür.',
      ],
      [
        'Uygulamalar nereye kurulur?',
        'Her uygulama kendi kurulum dosyasıyla, kendi varsayılan yerine kurulur (çoğunlukla %LOCALAPPDATA%\\Programs). Uygulama sayfasındaki "Kurulum yeri" klasörü açar.',
      ],
      [
        'Bir uygulamayı nasıl kaldırırım?',
        'Uygulamanın satırındaki ⋯ menüsünden "Kaldır". Uygulama açıksa önce kapatılır, sonra kendi kaldırma programı sessizce çalışır. Windows Ayarlar › Uygulamalar\'dan kaldırmak da aynı sonucu verir.',
      ],
      [
        "Robz Hub'ı kaldırırsam uygulamalar da gider mi?",
        'Hayır. Robz Hub yalnız bir yöneticidir; kurduğu uygulamalar bağımsızdır ve kendi başlarına çalışmaya devam eder.',
      ],
      [
        'Robz Hub kendini güncelliyor mu?',
        "Evet. Robz Hub'ın yeni sürümü Güncellemeler sayfasının en üstünde görünür; tek tıkla iner, kurulur ve Robz Hub yeniden açılır.",
      ],
      [
        'İndirme yarıda kesilirse?',
        'Yarım dosya silinir ve hiçbir şey kurulmaz. "Yeniden dene" ile baştan indirilir. İnternet yokken son bilinen sürümler gösterilir.',
      ],
      [
        'Hangi sistemlerde çalışır?',
        "64 bit Windows 10 ve 11 (arayüz Windows'ta zaten bulunan WebView2 ile) ve 64 bit Linux: .deb (Debian, Ubuntu), .rpm (Fedora, openSUSE) ve AppImage. Linux'ta Linux paketi olan uygulamalar kurulur; Robz Play ve Robz Vision yalnız Windows'ta çalışır.",
      ],
    ],

    finalTitle: "Robz Hub'ı indir, gerisini o halletsin.",
    finalText: 'Yeni sürüm ve uygulamalar ilk Discord sunucusunda duyurulur. Sorun yaşarsan ya da yeni bir uygulama önermek istersen orada ücretsiz destek de var.',
    discord: 'Discord',
    kick: 'Kick',
    issues: 'Sorun bildir',
    footerLicense: 'MIT lisanslı açık kaynak.',
    footerDisclaimer:
      'Robz Hub ve içindeki uygulamalar bağımsız projelerdir; Sony Interactive Entertainment, Rockstar Games ya da Take-Two Interactive ile bağlantılı değildir.',
    otherLang: 'English',
  },

  en: {
    lang: 'en',
    locale: 'en_US',
    title: 'Robz Hub: every Robz app in one place',
    description:
      'Install, update and launch Robz open-source apps such as Robz Play and Robz Vision with one click. Built with Rust, 2 MB, free.',
    skip: 'Skip to content',
    nav: { apps: 'Apps', usage: 'How to use', faq: 'FAQ', github: 'GitHub' },
    releasePill: "What's in this release",
    heroTitle: 'Every Robz app in one place.',
    heroLead:
      'Robz Hub installs, updates and launches the Robz open-source apps with one click. Play GTA VI at 60 FPS with Robz Play and read its subtitles in Turkish with Robz Vision: both from here.',
    download: 'Download for Windows',
    downloadLinux: 'Download for Linux',
    linux: 'For Linux:',
    otherFormats: 'Other formats:',
    portableLabel: 'Windows portable (no install)',
    debLabel: '.deb (Debian, Ubuntu)',
    rpmLabel: '.rpm (Fedora, openSUSE)',
    appimageLabel: 'AppImage (other distributions)',
    star: 'Star',
    starTitle: 'Star on GitHub',
    facts: ['Windows and Linux', 'Finds new apps on its own', 'Built with Rust', 'Free and open source'],
    shotAlt:
      'Robz Hub home page: the GTA VI bundle on top, Robz Play and Robz Vision below; Robz Vision has an update from 1.3.2 to 1.6.0.',

    whyTitle: 'Why Robz Hub?',
    whyLead:
      'Robz apps are made to be used together. Robz Hub lets you manage them from one place instead of finding, downloading and updating each one separately.',
    diffFile: 'robz-setup.diff',
    diffLabel: 'Setup before and with Robz Hub',
    diff: {
      del: [
        "Open Robz Play's page, find the right installer, download, install",
        "Open Robz Vision's page, pick Setup or Portable, download, install",
        'New version out? Check both repositories one by one',
      ],
      add: [
        'Install Robz Hub once',
        'Press "Install both": files download, get verified, install',
        'A notification arrives with each new version; press "Update"',
      ],
    },

    appsTitle: "What's inside",
    appsLead:
      'The list comes from a catalog on GitHub: when a new Robz app is released, it shows up without updating Robz Hub.',
    latest: 'Latest',
    appWebsite: 'Website',
    appRepo: 'Source code',
    bundleNote:
      'In Robz Hub, this box\'s "Install both" button downloads and installs both apps in turn; "Launch both" opens them together.',

    usageTitle: 'How to use',
    usageLead: 'Four steps from first install to playing.',
    steps: [
      {
        title: 'Download and install Robz Hub',
        text: 'Run RobzHub-Setup.exe. It needs no admin rights, installs for your user account only and takes a few seconds.',
        note: 'The file is not signed yet, so Windows may show "Windows protected your PC": More info › Run anyway.',
      },
      {
        title: 'Install the apps',
        text: '"Install both" on the home page sets up Robz Play and Robz Vision together; use "Install" in the list for one at a time. You see the download progress, speed and time left, and you can cancel.',
        shot: '06-home-en.png',
        shotAlt: 'Home page: the GTA VI bundle and the app list',
      },
      {
        title: 'Launch',
        text: 'Open installed apps with "Launch"; running ones get a green dot and "Close" shuts them. With the window closed, launch them from the tray icon menu.',
      },
      {
        title: 'Stay up to date',
        text: 'When a new version is out, a counter appears in the sidebar and Windows shows a notification. The Updates page shows what changed; "Update" does the rest and keeps your settings.',
        shot: '04-guncellemeler.png',
        shotAlt: 'Updates page: Robz Vision from 1.3.2 to 1.6.0 with its release notes',
      },
    ],

    detailsTitle: 'Details',
    details: [
      ['Small and fast', 'Written in Rust on Windows\' own WebView2 (Tauri). The installer is about 2 MB and it opens instantly.'],
      ['Every download is verified', "Installers come only from each app's own GitHub releases; if the SHA-256 digest doesn't match GitHub's, the file is deleted and never run."],
      ['Collects no data', 'No account, ads or usage statistics. Robz Hub only talks to GitHub and identifies itself as "RobzHub/version".'],
      ['Release notes and download counts', "Every release of every app with its date and download count; the ones newer than what you have are marked \"New\"."],
      ['Waits in the background', 'Optionally it runs in the tray, starts with Windows and tells you about new versions. All of it is in Settings.'],
      ['Turkish and English', 'The interface follows your Windows language; switch with the flag language picker in the sidebar.'],
    ],

    screensTitle: 'Screens',
    screens: [
      ['02-uygulama.png', 'App page: what it does, requirements, install location and actions.'],
      ['03-surum-notlari.png', 'Release notes: every release with its date, newest first.'],
      ['05-ayarlar.png', 'Settings: language, tray, start with Windows, notifications.'],
    ],
    openImage: 'Enlarge image',

    faqTitle: 'FAQ',
    faq: [
      ['Does Robz Hub cost anything?', 'No. Robz Hub and the apps in it are free and open source. No account or payment needed.'],
      [
        'What if I already installed the apps?',
        'Robz Hub finds installed apps and their versions on its own, from the Windows "Installed apps" list and the app folders. No need to reinstall; if a new version exists, an "Update" button appears. If you update an app from inside itself, Robz Hub notices within a few seconds.',
      ],
      [
        'Where do the apps get installed?',
        'Each app installs with its own installer to its usual place (mostly %LOCALAPPDATA%\\Programs). "Install location" on the app page opens the folder.',
      ],
      [
        'How do I uninstall an app?',
        'Choose "Uninstall" from the ⋯ menu on the app\'s row. If the app is running it is closed first, then its own uninstaller runs silently. Uninstalling from Windows Settings › Apps works the same.',
      ],
      ['If I uninstall Robz Hub, do the apps go too?', 'No. Robz Hub is only a manager; the apps it installed are independent and keep working.'],
      ['Does Robz Hub update itself?', 'Yes. A new Robz Hub version shows at the top of the Updates page; one click downloads and installs it and reopens Robz Hub.'],
      ['What if a download stops halfway?', 'The partial file is deleted and nothing is installed. "Try again" downloads it from the start. Offline, the last known versions are shown.'],
      [
        'Which systems are supported?',
        '64-bit Windows 10 and 11 (the interface uses WebView2, which Windows already has) and 64-bit Linux: .deb (Debian, Ubuntu), .rpm (Fedora, openSUSE) and AppImage. On Linux, apps that have a Linux package install; Robz Play and Robz Vision run on Windows only.',
      ],
    ],

    finalTitle: 'Download Robz Hub and let it handle the rest.',
    finalText: 'New releases and apps are announced first on the Discord server. If something goes wrong or you want to suggest an app, there is free support there too.',
    discord: 'Discord',
    kick: 'Kick',
    issues: 'Report an issue',
    footerLicense: 'Open source under the MIT license.',
    footerDisclaimer:
      'Robz Hub and the apps in it are independent projects, not affiliated with Sony Interactive Entertainment, Rockstar Games or Take-Two Interactive.',
    otherLang: 'Türkçe',
  },
};

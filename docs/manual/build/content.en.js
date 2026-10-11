// Text of the QRcode tool User Manual (English).
// Markup: **bold**, [[Key]], `code`. Figure names refer to shots/<lang>/ (see figures.js).

module.exports = {
  lang: 'en',
  font: 'Segoe UI',
  monoFont: 'Consolas',
  file: 'QRcode-tool-User-Manual-en',
  docTitle: 'QRcode tool User Manual',
  cover: {
    title: 'QRcode tool',
    subtitle: 'User Manual',
    tagline: 'Scan QR codes on your computer as easily as on your phone.',
    edition: 'Version 1.0.0   |   October 2026',
    platforms: 'For Windows 10 (21H2 or later) / Windows 11, macOS 13 or later and Linux',
  },
  back: {
    heading: 'Scan. Check. Then open.',
    body: [
      'QRcode tool is a free, open-source desktop scanner. It reads QR codes and barcodes from your screen, your webcam, the clipboard and image files, and checks every web address for common tricks before anything is opened.',
      'Everything happens on your computer. The program collects no usage data and never sends what you scan to the internet.',
    ],
    links: [
      ['Project page', 'https://github.com/Lrniey666/qrcode-tool'],
      ['Latest release', 'https://github.com/Lrniey666/qrcode-tool/releases/latest'],
      ['Report a problem', 'https://github.com/Lrniey666/qrcode-tool/issues'],
    ],
    license: 'The software is released under the MIT or Apache-2.0 license. This manual may be copied and adapted under the same terms.',
    trademark: 'QR Code is a registered trademark of DENSO WAVE INCORPORATED. QRcode tool is not affiliated with DENSO WAVE, Google or Microsoft.',
  },
  labels: {
    toc: 'Contents',
    chapter: (n) => `Chapter ${n}`,
    appendix: (l) => `Appendix ${l}`,
    figure: (id) => `Figure ${id}`,
    tip: 'Tip',
    note: 'Note',
    header: 'QRcode tool User Manual',
    tocHint: '(In Word, right-click the table of contents and choose Update Field to refresh the page numbers.)',
  },
  blocks: [
    // ───────────────────────── 1 ─────────────────────────
    { h1: 'Introducing QRcode tool' },
    { p: 'QRcode tool is a small desktop program that waits in the system tray and reads QR codes and barcodes from your screen. When you press its shortcut, the screen is frozen for a moment and every code on it gets a result label that shows what kind of content it holds and, for links, the real site name. Press [[Enter]] or click the label to open the link, join the Wi-Fi network or save the contact.' },
    { p: 'It can also read codes held up to your webcam, pictures on the clipboard and image files, and it includes a simple QR code generator.' },
    { h2: 'Main features' },
    {
      table: {
        cols: [26, 74],
        head: ['Feature', 'Description'],
        rows: [
          ['Scan whole screen', 'Freezes all monitors, finds every code automatically and labels it. Works with videos and games too.'],
          ['Select area', 'Drag a box around a single code. Use it for codes that are too small, blurry or tilted to be found automatically.'],
          ['Webcam', 'Hold a printed code or a phone screen up to the camera; results appear as soon as the code is recognised.'],
          ['Clipboard and image files', 'Reads a screenshot you just copied, or PNG, JPEG, BMP, GIF and WebP files.'],
          ['Understands the content', 'Recognises web links, Wi-Fi, contacts, email, phone numbers, text messages, map locations, 2-step verification keys, payment addresses and Taiwan e-invoices, and offers matching buttons.'],
          ['Link safety checks', 'Before a link is opened it is checked for unencrypted connections, disguised addresses, look-alike letters, IP addresses, short links and other common tricks. All checks run offline.'],
          ['Create QR codes', 'Turns text or a link into a QR code that you can copy or save as PNG or SVG.'],
          ['Floating toolbar and bubble', 'A toolbar that sits at the top of the screen (horizontal) or at the right or left edge (vertical), and that collapses into a small round bubble.'],
          ['Languages', 'English, Traditional Chinese, Simplified Chinese, Japanese, Korean, Spanish, Portuguese (Brazil), French, German and Russian.'],
        ],
      },
    },
    { h2: 'System requirements' },
    {
      table: {
        cols: [30, 70],
        head: ['Operating system', 'Requirements and notes'],
        rows: [
          ['Windows', 'Windows 10 21H2 or later, Windows 11. x64 and ARM64. No administrator rights are needed to install.'],
          ['macOS', 'macOS 13 or later (Apple silicon and Intel). Screen Recording permission is required for the first scan, Camera permission for webcam scanning.'],
          ['Linux', 'All features on X11. On Wayland, scanning is started through a keyboard shortcut you set up in your desktop settings (see Chapter 4).'],
          ['Hardware', 'Any ordinary office computer. A webcam is optional. The program uses less than about 30 MB of memory while idle.'],
        ],
      },
    },
    { h2: 'Privacy and safety principles' },
    {
      ul: [
        'Decoding and link checks run on your computer. Scanned content is never uploaded, and no web service is asked about the reputation of an address.',
        'The only network connection is the optional automatic update check, which asks GitHub whether a newer version exists.',
        'The scan history is off by default. Even when it is on, Wi-Fi passwords and 2-step verification keys are never stored.',
        'Only `http` and `https` addresses can be opened. Other kinds of links can only be copied; they are never handed to the system.',
        'A link with a warning always shows its details first and waits for you.',
      ],
    },
    { h2: 'Conventions used in this manual' },
    {
      ul: [
        'Buttons, options and window names are shown in **bold**, for example **Settings** or **Scan whole screen**.',
        'Keys are shown as key caps, for example [[Enter]] or [[Ctrl+Alt+Q]].',
        '**Settings › General** means: open the **Settings** window and choose the **General** page on the left.',
        'The pictures show Windows 11. macOS and Linux work the same way; only the window frames and system menus look different.',
      ],
    },

    // ───────────────────────── 2 ─────────────────────────
    { h1: 'Installation and first start' },
    { h2: 'Download and install (Windows)' },
    {
      ol: [
        'Open the release page on GitHub (https://github.com/Lrniey666/qrcode-tool/releases/latest).',
        'Download `QRcodeTool-win-x64-Setup.exe` for most computers, or `QRcodeTool-win-arm64-Setup.exe` for computers with an ARM processor.',
        'Run the downloaded file. QRcode tool is installed for your user account only and does not need administrator rights.',
        'When setup finishes, QRcode tool starts and shows the welcome screen.',
      ],
    },
    { tip: 'If you prefer not to install anything, download the `Portable.zip` file, unpack it and run the program directly. The portable version cannot update itself.' },
    { h2: 'If Windows shows “Windows protected your PC”' },
    { p: 'Current releases are not code-signed yet, so Windows SmartScreen may warn about an unknown publisher. This is common for new, unsigned programs.' },
    {
      ol: [
        'In the warning, click **More info**.',
        'Check that the app name is QRcode tool, then click **Run anyway**.',
      ],
    },
    { p: 'To make sure the download has not been altered, run the following command in PowerShell and compare the result with the `SHA256SUMS` file on the release page:' },
    { code: 'Get-FileHash .\\QRcodeTool-win-x64-Setup.exe -Algorithm SHA256' },
    { h2: 'macOS and Linux' },
    { p: '**macOS:** download and install the `.pkg` file. Releases are not notarized by Apple yet, so the first time, right-click the app and choose **Open**. On the first scan, allow QRcode tool under **System Settings › Privacy & Security › Screen Recording**, and under **Camera** if you want to use the webcam.' },
    { p: '**Linux:** download the `.AppImage` file, make it executable and run it. GNOME needs the “AppIndicator and KStatusNotifierItem Support” extension to show tray icons.' },
    { h2: 'The welcome screen' },
    { p: 'The welcome screen appears on the first start. It shows the three most common ways to scan, with their shortcuts, and explains that links never open by surprise.' },
    { fig: 'welcome', cap: 'The welcome screen on the first start', width: 11 },
    {
      ul: [
        '**Language:** follows your operating system by default. A change here takes effect immediately.',
        '**Start with the computer:** on by default so the shortcut is always ready. You can turn it off here or later in **Settings › General**.',
        '**Try it now:** closes the welcome screen and scans the whole screen once.',
        '**Close:** closes the welcome screen and shows the floating toolbar.',
      ],
    },
    { p: 'To see the welcome screen again, open **Settings › General** and click **Show the welcome screen again**.' },
    { h2: 'The tray icon' },
    { p: 'QRcode tool has no traditional main window. After it starts, its icon appears in the system tray at the right end of the taskbar, and every feature can be reached from there.' },
    {
      ul: [
        '**Click the icon:** scans the whole screen right away.',
        '**Right-click:** opens the menu (Figure 2-2).',
        '**Hover over the icon:** shows the current scan shortcut.',
      ],
    },
    { fig: 'tray-menu', cap: 'The tray icon menu', width: 11 },
    { tip: 'Windows may hide new tray icons behind the “^” arrow. Drag the icon onto the taskbar to keep it visible.' },

    // ───────────────────────── 3 ─────────────────────────
    { h1: 'Scanning the screen' },
    { h2: 'Scan the whole screen' },
    {
      ol: [
        'Make sure the web page, document, video or picture with the QR code is visible.',
        'Press [[Ctrl+Alt+Q]], click the tray icon, or click **Scan whole screen** on the floating toolbar.',
        'The screen freezes and darkens. After a short scan sweep, every code found gets a result label.',
        'Press [[Enter]] to open the selected code, or click the label you want.',
        'Press [[Esc]] or right-click to leave the scan screen.',
      ],
    },
    { fig: 'scan-screen', cap: 'Scan whole screen: every code gets a label; the first one is selected' },
    { p: 'The status chip at the top shows how many codes were found. The colour of the brackets around each code tells you the result of the checks:' },
    {
      table: {
        cols: [22, 78],
        head: ['Bracket colour', 'Meaning'],
        rows: [
          ['White', 'Nothing suspicious was found.'],
          ['Accent (blue)', 'The selected code. [[Enter]] opens it.'],
          ['Yellow', 'Something deserves attention, for example an unencrypted http link or a short link. The reason is shown under the label.'],
          ['Red', 'The link looks deceptive, for example a disguised address or a domain that mixes alphabets.'],
        ],
      },
    },
    { p: 'Because the screen is frozen, codes in videos, presentations and games can be read too. With several monitors, all of them are scanned at the same time.' },
    { h2: 'Select an area' },
    { p: 'When a code is small, blurry or tilted, or when there are many codes on the screen, draw a box around just the one you need. The selected area is enlarged before it is read, which works better than a whole-screen scan for difficult codes.' },
    {
      ol: [
        'Press [[Ctrl+Alt+A]], or choose **Select area** from the tray menu or the floating toolbar.',
        'The screen freezes and the pointer becomes a crosshair. Hold down the left mouse button and drag a box around the code.',
        'Release the mouse button. Only the content inside the box is read.',
      ],
    },
    { fig: 'scan-region', cap: 'Select area: the size of the box is shown while you drag' },
    { tip: 'Make the box a little larger than the code; a margin of empty space around it helps. If the message “No code in that area” appears, draw a slightly bigger box.' },
    { h2: 'The toolbar on the scan screen' },
    { p: 'The scan screen has a row of buttons at the top. From left to right: **Scan whole screen**, **Select area**, **Scan with camera**, **Scan image from clipboard**, **Open image…**, **Create QR code…**, **History**, **Settings** and **Close**. You can switch methods without leaving the scan screen, for example from the whole screen to a selected area. Hover over a button to see its description and shortcut.' },
    { h2: 'Keyboard' },
    { p: 'The scan screen can be used entirely with the keyboard:' },
    {
      table: {
        cols: [30, 70],
        head: ['Key', 'Action'],
        rows: [
          ['[[Enter]]', 'Open the selected code.'],
          ['[[Tab]], [[→]], [[↓]]', 'Select the next code.'],
          ['[[Shift+Tab]], [[←]], [[↑]]', 'Select the previous code.'],
          ['[[C]]', 'Copy the selected code.'],
          ['[[F]] or [[F5]]', 'Search the whole screen again.'],
          ['[[Esc]]', 'Cancel and leave the scan screen (right-click works too).'],
        ],
      },
    },
    { h2: 'When no code is found' },
    {
      ul: [
        'Use **Select area** and draw a box around the code only; it is enlarged before it is read.',
        'If the code moves quickly in a video, pause the video first.',
        'Move other windows, the mouse pointer or anything else that covers part of the code.',
        'Check that **Settings › Scanning › Also read barcodes** is on if you want to read barcodes and formats such as Data Matrix.',
        'If the code is very small on the screen (less than about 40 pixels wide), zoom in on the page or picture first.',
      ],
    },

    // ───────────────────────── 4 ─────────────────────────
    { h1: 'Other ways to scan' },
    { h2: 'Webcam' },
    { p: 'Use the webcam for codes on paper, business cards or another phone’s screen.' },
    {
      ol: [
        'Press [[Ctrl+Alt+W]], or choose **Scan with camera**.',
        'Hold the code up to the camera at a distance of about 15 to 30 cm.',
        'When a code is recognised, brackets appear around it and its result label is placed in a steady list at the bottom of the picture.',
        'Click the label, or select it with [[Tab]] and press [[Enter]].',
      ],
    },
    { fig: 'camera', cap: 'Scan with camera: results stay in a steady list at the bottom', width: 13 },
    {
      ul: [
        'A code has to be seen in several frames in a row before it is listed, which prevents misreads.',
        'After a code leaves the picture, its result stays listed for a while (8 seconds by default) so you can still click it. Change the time in **Settings › Camera**.',
        'If more than one camera is connected, switch between them with the list at the bottom of the window.',
        'The **Mirror** switch only changes how the picture is shown; recognition works either way.',
        'By default the camera window closes after a code has been opened. You can change this in **Settings › Camera**.',
      ],
    },
    { note: 'If the camera does not start, close other programs that may be using it (video calls, for example) and allow QRcode tool to use the camera in your system’s privacy settings.' },
    { h2: 'Image on the clipboard' },
    {
      ol: [
        'Take a screenshot of the code with [[Win+Shift+S]] or another screenshot tool, or copy a picture from a web page.',
        'Choose **Scan image from clipboard**.',
        'The picture is shown in a preview window, and every code found gets a result label.',
      ],
    },
    { p: 'If there is no picture on the clipboard, the program asks you to copy a screenshot or picture first.' },
    { h2: 'Image files' },
    {
      ol: [
        'Choose **Open image…**.',
        'Select a PNG, JPEG, BMP, GIF or WebP file.',
        'Click a result label in the preview window. The buttons at the bottom open another picture or scan the clipboard instead.',
      ],
    },
    { fig: 'image-preview', cap: 'The preview window: labels are placed on the codes in the picture', width: 13 },
    { h2: 'Windows right-click menu' },
    { p: 'On Windows, QRcode tool adds the following entries to the desktop and image file right-click menus. On Windows 11, click **Show more options** first.' },
    {
      table: {
        cols: [42, 58],
        head: ['Menu entry', 'Where and what it does'],
        rows: [
          ['QRcode tool Scan screen', 'Desktop background. Scans the whole screen.'],
          ['QRcode tool Scan selection', 'Desktop background. Select an area.'],
          ['QRcode tool Scan with camera', 'Desktop background. Opens the webcam.'],
          ['QRcode tool Toolbar', 'Desktop background. Shows or hides the floating toolbar.'],
          ['QRcode tool Open image', 'Image files (PNG, JPEG, BMP, GIF, WebP). Scans the picture in the preview window.'],
        ],
      },
    },
    { p: 'Turn the entries off in **Settings › General › Right-click menu**. Uninstalling removes them as well.' },
    { h2: 'Command line' },
    { p: 'Every scan function can be started with a command. If the program is already running, the command is passed to it. This is especially useful on Linux with Wayland: assign the commands to custom shortcuts in your desktop’s keyboard settings.' },
    {
      table: {
        cols: [42, 58],
        head: ['Command', 'Action'],
        rows: [
          ['`qrcode-tool`', 'Start the program, or show the floating toolbar if it is already running.'],
          ['`qrcode-tool scan-screen`', 'Scan the whole screen.'],
          ['`qrcode-tool scan-region`', 'Freeze the screen and select an area.'],
          ['`qrcode-tool scan-camera`', 'Scan with the webcam.'],
          ['`qrcode-tool scan-clipboard`', 'Scan the picture on the clipboard.'],
          ['`qrcode-tool scan-file FILE`', 'Scan one image file in the preview window.'],
          ['`qrcode-tool toolbar`', 'Show or hide the floating toolbar.'],
          ['`qrcode-tool bubble`', 'Show or hide the floating bubble.'],
          ['`qrcode-tool generate "TEXT"`', 'Open the QR code generator with the text filled in.'],
          ['`qrcode-tool settings`', 'Open Settings.'],
          ['`qrcode-tool decode FILE… [--json]`', 'Print the codes in image files to the terminal without starting the UI.'],
          ['`qrcode-tool quit`', 'Quit the running program.'],
        ],
      },
    },

    // ───────────────────────── 5 ─────────────────────────
    { h1: 'Results and details' },
    { h2: 'Result labels' },
    { p: 'Every code found gets a dark result label. From left to right it shows:' },
    {
      ul: [
        '**Icon:** the kind of content, for example a globe for a link, a fan for Wi-Fi or a person for a contact. Links with a warning show a warning or danger symbol instead.',
        '**Main text:** for links, the real site name rather than the full address; for Wi-Fi, the network name; for contacts, the name.',
        '**Second line:** the kind of content, or the main reason for a warning.',
        '**› button:** always opens the detail card, whatever the content.',
      ],
    },
    { h2: 'What Enter or a click does' },
    {
      table: {
        cols: [36, 64],
        head: ['Content', 'Action'],
        rows: [
          ['Link without warnings', 'Opens in your default browser.'],
          ['Link with a warning', 'Shows the detail card first; the link opens only after you confirm.'],
          ['Everything else', 'Shows the detail card, where you choose what to do.'],
        ],
      },
    },
    { h2: 'The detail card' },
    { p: 'The detail card lists every field in the code and offers buttons that match the content. The copy button next to a field copies just that field. Sensitive fields such as passwords are hidden as “••••”; the lock button shows them for a moment. Click outside the card or click **Close** to return to the results.' },
    { fig: 'detail-wifi', cap: 'Detail card of a Wi-Fi code: Connect joins the network', width: 12 },
    { h2: 'Buttons for each kind of content' },
    {
      table: {
        cols: [30, 70],
        head: ['Kind', 'Buttons'],
        rows: [
          ['Link', '**Open**, **Copy**. Links without warnings to sites you have not trusted yet also offer **Always allow this site**.'],
          ['Wi-Fi network', '**Connect** (the system creates the network profile), **Copy password**.'],
          ['Contact (vCard, MeCard)', '**Save contact** (saves a .vcf file you can import into your address book), **Copy**.'],
          ['Email', '**Write email** (opens your default mail program), **Copy**.'],
          ['Phone number', '**Call** (handed to the system’s calling app), **Copy**.'],
          ['Text message', '**Send message**, **Copy**.'],
          ['Location', '**Open map**, **Copy**.'],
          ['2-step verification key', 'Only the service name and account are shown; **Copy** is available. The secret itself is never shown or stored.'],
          ['Payment address', '**Copy**. Always double-check a payment address before sending money.'],
          ['E-invoice', '**Copy invoice number**, **Copy**.'],
          ['Blocked link', 'Links other than `http` and `https` (for example `file:` or `javascript:`) can only be copied, never opened.'],
          ['Text and barcodes', '**Copy**. For barcodes the format, for example EAN-13, is shown as well.'],
        ],
      },
    },
    { h2: 'Taiwan e-invoices' },
    { p: 'A Taiwan e-invoice has two QR codes side by side. The left code holds the invoice number, date, amounts and some of the items; the right code holds the remaining items. When **Scan whole screen** or **Open image…** finds both codes, they are merged into one invoice and the detail card lists the invoice number, date, random code, total, tax, seller and buyer IDs and all items. If only the left code is found, the card reminds you to scan the right code as well.' },
    { h2: 'Sensitive data' },
    { p: '2-step verification keys and Wi-Fi passwords are sensitive. Even with the scan history or **Copy the result automatically** turned on, they are never written to the history, never copied automatically and never written to the program’s log files.' },

    // ───────────────────────── 6 ─────────────────────────
    { h1: 'Link safety' },
    { p: 'Malicious QR codes often lead to fake websites. QRcode tool analyses every web address on your computer before it shows the result, and marks what it finds directly on the result label and the detail card. Nothing is looked up online, and no address is sent to any service.' },
    { h2: 'Check the site name before opening' },
    { p: 'The result label shows the real site name contained in the address. Before you open any link, make sure this is the site you expect. Links with a warning show their detail card first, with a coloured notice at the top that explains what to watch out for.' },
    { fig: 'detail-warning', cap: 'A link with warnings: an unencrypted short link shows the reasons and waits for you', width: 12 },
    { h2: 'Warning levels' },
    {
      table: {
        cols: [18, 82],
        head: ['Level', 'What the program does'],
        rows: [
          ['Info', 'For example a very long address. Listed on the detail card only; [[Enter]] opens the link directly.'],
          ['Warning', 'Yellow brackets and icon. [[Enter]] or a click shows the detail card first; you then click **Open**.'],
          ['Danger', 'Red brackets and icon. **Open** is no longer the default button, and after clicking it you must confirm with **Open anyway**.'],
        ],
      },
    },
    { h2: 'What is checked' },
    {
      table: {
        cols: [30, 12, 58],
        head: ['Message shown', 'Level', 'Explanation and advice'],
        rows: [
          ['Not encrypted (http)', 'Warning', 'The connection is not encrypted, so data can be read or changed on the way. Do not enter passwords or payment details on such pages.'],
          ['Disguised address', 'Danger', 'A trusted-looking name is placed before an “@”, but the link really goes to the site after it, for example `https://bank.example.com@login-check.example.net/`.'],
          ['International domain name', 'Warning', 'The domain contains non-Latin letters. The card also shows the real encoded address (starting with `xn--`) so you can check it.'],
          ['Look-alike letters', 'Danger', 'One part of the domain mixes alphabets, for example a Cyrillic “а” in place of a Latin “a”. This is a common trick of fake websites.'],
          ['Raw IP address', 'Warning', 'The address uses numbers instead of a site name, which ordinary websites rarely do.'],
          ['Unusual port', 'Warning', 'The address specifies a non-standard port number.'],
          ['Short link', 'Warning', 'Services such as bit.ly hide the real destination. The link is not expanded online, because that would tell the service what you scanned. Open short links only from sources you trust.'],
          ['Very long address', 'Info', 'Very long addresses can push the real site name out of view.'],
          ['Many encoded characters', 'Info', 'Many “%” escapes can be used to hide what an address contains.'],
          ['Many subdomains', 'Info', 'For example `login.bank.example.com.secure-check.example.net`, which can look like a bank’s site at first glance.'],
        ],
      },
    },
    { h2: 'Deceptive links' },
    { p: 'For links rated **Danger**, the detail card says in red that the link looks deceptive and lists the reasons. Rely on the real site name shown on the card, not on the text printed next to the code.' },
    { fig: 'detail-danger', cap: 'A disguised address: it looks like a bank but really goes to login-check.example.net', width: 12 },
    { p: 'If you are sure you want to continue, click **Open**. The card asks once more; only **Open anyway** really opens the page.' },
    { h2: 'Trusted sites and automatic opening' },
    { p: 'You can add sites you use often, such as your school’s or company’s domain, to the trusted sites. A link then opens straight after the scan, without waiting for [[Enter]], when all of these are true:' },
    {
      ul: [
        'Only one code was found.',
        'The link has no warnings.',
        'The site (or one of its subdomains) is on the trusted list.',
        '**Settings › Scanning › Open trusted sites without asking** is on (the default).',
      ],
    },
    { p: 'Add a site with **Always allow this site** on the detail card of a link without warnings, or type the domain in **Settings › Scanning**.' },
    { fig: 'detail-link', cap: 'A link without warnings: open it, or add the site to the trusted list', width: 12 },

    // ───────────────────────── 7 ─────────────────────────
    { h1: 'Floating toolbar and bubble' },
    { p: 'The floating toolbar stays on top of other windows so every function is one click away. It never appears in scanned screenshots. Show or hide it with **Floating toolbar** in the tray menu, the shortcut [[Ctrl+Alt+E]] or the command `qrcode-tool toolbar`.' },
    { h2: 'Toolbar buttons' },
    { fig: 'toolbar-row', cap: 'Horizontal toolbar: hover over a button to see its name and shortcut underneath', width: 11 },
    {
      table: {
        cols: [34, 66],
        head: ['Button', 'Action'],
        rows: [
          ['(dotted handle at the start)', 'Hold and drag to move the toolbar.'],
          ['Scan whole screen', 'Same as [[Ctrl+Alt+Q]].'],
          ['Select area', 'Same as [[Ctrl+Alt+A]].'],
          ['Scan with camera', 'Same as [[Ctrl+Alt+W]]. Greyed out when no camera is found.'],
          ['Scan image from clipboard', 'Reads the picture on the clipboard.'],
          ['Open image…', 'Choose an image file and scan it.'],
          ['Create QR code…', 'Opens the QR code generator.'],
          ['History', 'Opens **Settings › Privacy & history**.'],
          ['Settings', 'Opens Settings.'],
          ['User manual (“?” button)', 'Opens the PDF user manual on GitHub in a new browser tab: the Traditional Chinese edition when the UI is in Chinese, the English edition otherwise.'],
          ['Collapse to bubble', 'Hides the toolbar and shows the small round bubble instead.'],
          ['Close toolbar', 'Hides the toolbar (the bubble does not appear). The program keeps running in the tray.'],
        ],
      },
    },
    { h2: 'Vertical toolbar (right or left)' },
    { p: 'Besides the default horizontal layout at the top of the screen, the toolbar can be a vertical column at the right or left edge of the screen. The column takes up less width, which suits wide screens and keeps the top of web pages uncovered.' },
    {
      figs: [
        ['toolbar-right', 'Vertical, on the right (the default vertical position)'],
        ['toolbar-left', 'Vertical, on the left'],
      ],
      cap: 'Vertical toolbar: the name and shortcut of the hovered button appear below the buttons',
    },
    { p: 'To change the layout:' },
    {
      ol: [
        'Open **Settings** and choose **General**.',
        'In the **Floating toolbar and bubble** section, find **Toolbar layout**.',
        'Choose **Horizontal, at the top**, **Vertical, on the right** or **Vertical, on the left**.',
      ],
    },
    { p: 'The change takes effect immediately. The toolbar moves to the default place of the new layout: centred at the top for the horizontal layout, vertically centred at the chosen edge for the vertical one. You can still drag it anywhere afterwards.' },
    { h2: 'Moving the toolbar' },
    { p: 'Hold the dotted handle at one end of the toolbar and drag. The program remembers the last position and shows the toolbar there next time. If that position is no longer on any monitor, for example after unplugging an external display, the toolbar returns to its default place.' },
    { h2: 'The floating bubble' },
    { p: 'The floating bubble is the collapsed toolbar. It takes up only a small circle and appears at the right edge of the screen by default.' },
    { fig: 'bubble', cap: 'The floating bubble', width: 8 },
    {
      table: {
        cols: [30, 70],
        head: ['Action', 'Result'],
        rows: [
          ['Click', 'Expands into the full floating toolbar.'],
          ['Hold for about half a second', 'Scans the whole screen.'],
          ['Right-click', 'Opens a menu: **Scan whole screen**, **Select area**, **Scan with camera**, **Scan image from clipboard**, **Show toolbar**, **Hide bubble**.'],
          ['Drag', 'Moves the bubble.'],
        ],
      },
    },
    { p: 'To show the bubble every time the program starts, turn on **Settings › General › Show the bubble when the app starts**. The toolbar and the bubble are never shown together.' },
    { h2: 'Hiding during full-screen apps' },
    { p: 'On Windows, the toolbar and bubble hide while you watch a full-screen video, give a presentation or play a game, and come back when you leave full-screen mode. Turn this off in **Settings › General › Hide during full-screen apps**.' },

    // ───────────────────────── 8 ─────────────────────────
    { h1: 'Creating QR codes' },
    { p: 'The QR code generator turns a link or a piece of text into a QR code that you can scan with a phone or place in documents and presentations.' },
    {
      ol: [
        'Choose **Create QR code…** from the tray menu, the floating toolbar or the scan screen.',
        'Type or paste the content into **Text or link**. The QR code appears on the right straight away.',
        'Choose an **Error correction** level if needed.',
        'Click **Copy image** to copy the QR code to the clipboard, or **Save PNG…** or **Save SVG…** to save it as a file.',
      ],
    },
    { fig: 'generator', cap: 'The QR code generator', width: 13 },
    { h2: 'Error correction' },
    { p: 'A higher level lets the code be read even when part of it is damaged or covered by a logo, but makes the code denser and larger.' },
    {
      table: {
        cols: [36, 64],
        head: ['Level', 'When to use it'],
        rows: [
          ['Low — smallest code', 'Lots of content that will only be shown on screen.'],
          ['Medium — recommended', 'The default for general use.'],
          ['Quartile', 'Codes printed on things that may get scuffed.'],
          ['High — survives damage or a logo', 'When a logo will be placed in the middle of the code, or maximum robustness is needed.'],
        ],
      },
    },
    { note: 'A QR code can only hold a limited amount of content. If “Too much text for one QR code” appears, shorten the content or choose a lower error correction level.' },
    { p: '**PNG** is convenient for documents and slides; **SVG** is a vector format that stays sharp when printed large. Press [[Esc]] to close the generator.' },

    // ───────────────────────── 9 ─────────────────────────
    { h1: 'Settings' },
    { p: 'Choose **Settings** from the tray menu, the floating toolbar or the scan screen. Every change is saved and applied immediately; there is no OK button.' },
    { h2: 'General' },
    { fig: 'settings-general', cap: 'Settings › General', width: 14 },
    {
      table: {
        cols: [34, 66],
        head: ['Setting', 'Description'],
        rows: [
          ['Language', 'Language of menus and messages. Follows the operating system by default.'],
          ['Theme', '**Use system setting**, **Light** or **Dark**. The system accent colour is applied automatically.'],
          ['Start with the computer', 'Starts quietly in the background when you sign in, so the shortcut always works.'],
          ['Right-click menu', '(Windows only) Adds QRcode tool entries to the desktop and image file right-click menus.'],
          ['Toolbar layout', '**Horizontal, at the top**, **Vertical, on the right** or **Vertical, on the left**. See Chapter 7.'],
          ['Show the bubble when the app starts', 'Shows the floating bubble on start-up.'],
          ['Hide during full-screen apps', '(Windows only) Hides the toolbar and bubble during full-screen videos, presentations and games.'],
          ['Check for updates automatically', 'Asks GitHub once a day whether a new version exists. Nothing is installed without asking you.'],
          ['Show the welcome screen again', 'Opens the first-run welcome screen.'],
        ],
      },
    },
    { h2: 'Scanning' },
    { fig: 'settings-scanning', cap: 'Settings › Scanning', width: 14 },
    {
      table: {
        cols: [34, 66],
        head: ['Setting', 'Description'],
        rows: [
          ['Copy the result automatically', 'When exactly one code is found, its content is put on the clipboard. Sensitive content is never copied. Off by default.'],
          ['Play a sound when a code is found', 'Plays a short sound. Off by default.'],
          ['Also read barcodes', 'Reads EAN, UPC, Code 128, Data Matrix, PDF417, Aztec and more in addition to QR codes. On by default.'],
          ['Open trusted sites without asking', 'Opens links to trusted sites automatically under the conditions listed in Chapter 6.'],
          ['Trusted sites', 'Type a domain (for example example.com) and click **Add**. Subdomains are trusted too. The bin icon removes an entry.'],
        ],
      },
    },
    { h2: 'Camera' },
    { fig: 'settings-camera', cap: 'Settings › Camera', width: 14 },
    {
      table: {
        cols: [34, 66],
        head: ['Setting', 'Description'],
        rows: [
          ['Camera', 'The webcam to use.'],
          ['Mirror the picture', 'Shows the camera like a mirror. Recognition works either way.'],
          ['Keep results on screen (seconds)', 'How long (0 to 60 seconds) a result stays listed after its code leaves the picture. With 0, only codes currently in view are listed.'],
          ['Close the camera after a code is opened', 'Closes the camera window once a code has been opened or used.'],
        ],
      },
    },
    { h2: 'Shortcuts' },
    { fig: 'settings-shortcuts', cap: 'Settings › Shortcuts', width: 14 },
    {
      table: {
        cols: [34, 22, 44],
        head: ['Action', 'Default', 'Description'],
        rows: [
          ['Scan whole screen', '[[Ctrl+Alt+Q]]', 'Freezes and scans all monitors.'],
          ['Select area', '[[Ctrl+Alt+A]]', 'Freezes the screen so you can drag a box.'],
          ['Scan with camera', '[[Ctrl+Alt+W]]', 'Opens the webcam window.'],
          ['Show or hide the toolbar', '[[Ctrl+Alt+E]]', 'Toggles the floating toolbar.'],
        ],
      },
    },
    { p: 'To change a shortcut:' },
    {
      ol: [
        'Click the shortcut you want to change. The button shows **Press keys… (Esc to cancel)**.',
        'Press the new key combination. It must include at least one of [[Ctrl]], [[Alt]], [[Shift]] or [[Win]] (on a Mac, [[Cmd]]).',
        'The new shortcut takes effect immediately.',
      ],
    },
    { p: 'If another app already uses the combination, the row says “Already used by another app — choose another one.” Pick a different combination. **Restore defaults** brings back all default shortcuts. The **While scanning** section at the bottom lists the keys of the scan screen, the same as in Chapter 3.' },
    { h2: 'Privacy & history' },
    { fig: 'settings-privacy', cap: 'Settings › Privacy & history (with the history turned on)', width: 14 },
    {
      table: {
        cols: [34, 66],
        head: ['Setting', 'Description'],
        rows: [
          ['Keep a scan history', 'Stored only on this computer. Wi-Fi passwords and 2-step verification keys are never saved. Off by default.'],
          ['Number of entries to keep', '5 to 500 entries. The oldest entries are removed first.'],
          ['Recent scans', 'Newest first. Links without warnings can be opened again with **Open**; every entry can be copied with **Copy**.'],
          ['Clear history', 'Deletes the whole history.'],
        ],
      },
    },
    { h2: 'About' },
    { fig: 'settings-about', cap: 'Settings › About', width: 14 },
    {
      ul: [
        'Shows the current version. **Check for updates** checks right away.',
        'Links to the source code, problem reports, the privacy statement and open-source licenses.',
        '**Open the settings folder** and **Open the log folder** open those folders directly, for backing up your settings or attaching log files to a problem report.',
      ],
    },

    // ───────────────────────── 10 ─────────────────────────
    { h1: 'Updating and uninstalling' },
    { h2: 'Checking for and installing updates' },
    { p: 'Copies installed with the setup program can update themselves. With **Check for updates automatically** on, the program asks GitHub once a day. You can also click **Check for updates** in the tray menu or in **Settings › About** at any time.' },
    { fig: 'message-update', cap: 'The message shown when a new version is available', width: 10 },
    {
      ol: [
        'When the **Update available** message appears, click **Update now**.',
        'The new version is downloaded in the background, and the program restarts when it is done.',
        'Click **Later** if you do not want to update now.',
      ],
    },
    { p: 'Updates are only installed after you click **Update now**; the program never installs anything on its own.' },
    { h2: 'Portable version' },
    { p: 'The portable version (`Portable.zip`) does not update itself; checking for updates shows “Updates are not available”. Download the new version from the GitHub release page and replace the old files. Your settings are kept.' },
    { h2: 'Uninstalling' },
    {
      ol: [
        'Open Windows **Settings › Apps › Installed apps**.',
        'Find QRcode tool, click **…** and choose **Uninstall**.',
        'The start-up entry and the right-click menu entries are removed as well.',
      ],
    },
    { p: 'To remove your settings and history too, open the settings folder and the log folder from **Settings › About** before uninstalling, and delete them.' },
    { h2: 'Where settings and logs are stored' },
    {
      table: {
        cols: [18, 82],
        head: ['System', 'Location'],
        rows: [
          ['Windows', 'Settings: `%APPDATA%\\Lrniey666\\qrcode-tool\\config`\nHistory and logs: `%LOCALAPPDATA%\\Lrniey666\\qrcode-tool\\data`'],
          ['macOS', '`~/Library/Application Support/io.github.Lrniey666.qrcode-tool`'],
          ['Linux', 'Settings: `~/.config/qrcode-tool`\nHistory and logs: `~/.local/share/qrcode-tool`'],
        ],
      },
    },
    { p: 'The settings file `settings.toml` is plain text. Copy it to move your settings to another computer.' },

    // ───────────────────────── 11 ─────────────────────────
    { h1: 'Troubleshooting' },
    {
      qa: [
        ['Nothing happens when I press the shortcut.', 'Another app may already use it. Open **Settings › Shortcuts**; if the row says “Already used by another app”, choose another combination. Some games and remote-desktop programs capture all keys while they are in front.'],
        ['The scan finds no code.', 'Use **Select area** and draw a box around the code only, or zoom in on the page or picture first. See “When no code is found” in Chapter 3.'],
        ['Why did the link not open right away?', 'A link with a warning always shows its details first so you can check the real site. Only links to trusted sites without any warning open automatically, and only when they are the single code found.'],
        ['It says “Short link — the real destination is hidden”.', 'Short-link services hide the real destination. The program does not look it up online, because that would reveal what you scanned. Open short links only from sources you trust.'],
        ['Where is the main window?', 'There is none on purpose. Use the tray icon, the shortcut, the floating toolbar or the floating bubble.'],
        ['The toolbar has disappeared.', 'Press [[Ctrl+Alt+E]] or tick **Floating toolbar** in the tray menu. With **Hide during full-screen apps** on, the toolbar is hidden while a full-screen program is in front.'],
        ['The camera does not start.', 'Close other programs that use the camera and check that your system’s privacy settings allow QRcode tool to use it. With several cameras, choose the right one in **Settings › Camera**.'],
        ['Wi-Fi does not connect automatically.', 'Some networks (for example enterprise authentication) cannot be set up by the program. Click **Copy password** on the detail card and connect from the system’s Wi-Fi menu.'],
        ['My antivirus program warns about it.', 'New unsigned programs that register shortcuts, read the screen and start with the computer are sometimes flagged by behaviour-based detection. QRcode tool uses standard system interfaces and its source code is public. Please report the false positive to your antivirus vendor, or open an issue on GitHub.'],
        ['On macOS the screenshot is black.', 'Allow QRcode tool under **System Settings › Privacy & Security › Screen Recording** and restart the program. macOS may ask again after system updates.'],
        ['On Linux with Wayland the shortcut does nothing.', 'Wayland does not allow programs to register global shortcuts. Add a custom shortcut in your desktop’s keyboard settings that runs `qrcode-tool scan-screen` or another command.'],
        ['How do I open this manual again?', 'Click the **?** button (**User manual**) on the floating toolbar; the PDF edition on GitHub opens in a new browser tab. The Word and PDF files can also be downloaded from the GitHub release page.'],
        ['How do I report a problem?', 'Click **Report a problem or suggest an idea** in **Settings › About** and describe your system, the steps and the messages you saw. You can attach files from the log folder; logs contain events and errors only, never scanned content.'],
      ],
    },

    // ───────────────────────── Appendices ─────────────────────────
    { h1: 'Shortcuts and keys at a glance', appendix: true },
    {
      table: {
        cols: [30, 26, 44],
        head: ['Where', 'Key', 'Action'],
        rows: [
          ['Anywhere', '[[Ctrl+Alt+Q]]', 'Scan whole screen'],
          ['Anywhere', '[[Ctrl+Alt+A]]', 'Select area'],
          ['Anywhere', '[[Ctrl+Alt+W]]', 'Scan with camera'],
          ['Anywhere', '[[Ctrl+Alt+E]]', 'Show or hide the floating toolbar'],
          ['Scan screen, preview', '[[Enter]]', 'Open the selected code'],
          ['Scan screen, preview', '[[Tab]] / [[Shift+Tab]]', 'Select the next / previous code'],
          ['Scan screen, preview', '[[C]]', 'Copy the selected code'],
          ['Scan screen', '[[F]] or [[F5]]', 'Search the whole screen again'],
          ['Scan screen, preview, generator', '[[Esc]]', 'Close'],
        ],
      },
    },
    { p: 'The global shortcuts can be changed in **Settings › Shortcuts**. The table shows the defaults.' },
    { h1: 'Supported codes', appendix: true },
    {
      table: {
        cols: [30, 70],
        head: ['Group', 'Formats'],
        rows: [
          ['Always read', 'QR Code, Micro QR Code, rMQR Code'],
          ['With “Also read barcodes”', 'EAN-13, EAN-8, UPC-A, UPC-E, Code 128, Code 39, Code 93, Codabar, ITF, DataBar, Data Matrix, PDF417, Aztec and more'],
        ],
      },
    },
    {
      table: {
        cols: [30, 70],
        head: ['Recognised content', 'Example'],
        rows: [
          ['Web link', '`https://example.com/`'],
          ['Wi-Fi', '`WIFI:T:WPA;S:network;P:password;;`'],
          ['Contact', 'vCard (`BEGIN:VCARD`), MeCard (`MECARD:`)'],
          ['Email', '`mailto:`, `MATMSG:`'],
          ['Phone and text message', '`tel:`, `sms:`, `SMSTO:`'],
          ['Location', '`geo:25.0330,121.5654`'],
          ['2-step verification key', '`otpauth://`'],
          ['Payment address', '`bitcoin:`, `ethereum:` and others'],
          ['Taiwan e-invoice', 'The two QR codes on the invoice'],
        ],
      },
    },
  ],
};

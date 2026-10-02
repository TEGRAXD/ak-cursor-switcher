graph TD
    A[Launch App: main.rs] --> B[Init CommonControls v6 & Win32 Window]
    B --> C[WM_CREATE]
    C --> D[Load Preferences: settings.ini]
    C --> E[Scan Schemes: HKCU & HKLM]
    C --> F[Create Tray Icon & Start 1s Timer]

    F --> G[1s Polling: WM_TIMER]
    G --> H{Auto-switch Enabled?}
    H -- No --> G
    H -- Yes --> I[Scan Processes: Toolhelp32Snapshot]

    I --> J{Target Running?}
    J -- Started Now --> K[cursor::apply_scheme_and_size]
    J -- Exited Now --> L[cursor::restore_scheme]
    J -- No Change --> G

    K --> G
    L --> G

    M[User UI / Tray Interaction] --> N[Change Scheme / Size / Toggle]
    N --> O[Apply Cursor Instantly]
    N --> P[Auto-save settings.ini]

    Q[Window Close / Exit] --> R[Auto-save settings.ini]
    R --> S[Remove Tray Icon & Delete GDI Font]
    S --> T[PostQuitMessage: Exit Process]

use crate::config::settings::Language;

/// All user-facing strings in the UI.
pub struct Messages {
    // Agent events
    pub agent_ready: &'static str,
    pub agent_completed: &'static str,
    pub agent_error: &'static str,

    // Task events
    pub task_created: &'static str,
    pub task_assigned: &'static str,
    pub task_completed: &'static str,

    // Team task phases
    pub team_planning: &'static str,
    pub team_executing: &'static str,
    pub team_synthesizing: &'static str,
    pub team_completed: &'static str,

    // Communication
    pub message_routed: &'static str,
    pub broadcast_sent: &'static str,

    // Commands
    pub unknown_command: &'static str,
    pub usage_send: &'static str,
    pub usage_broadcast: &'static str,
    pub usage_teamtask: &'static str,

    // UI labels
    pub settings_saved: &'static str,
    pub settings_save_error: &'static str,
    pub settings_tab_title: &'static str,
    pub log_tab_title: &'static str,
    pub dashboard_title: &'static str,
    pub agent_detail_title: &'static str,
    pub tasks_title: &'static str,
    pub office_title: &'static str,

    // Help
    pub help_title: &'static str,

    // Input
    pub command_title: &'static str,
    pub prompt_title: &'static str,
    pub task_input_title: &'static str,

    // Team task initiated
    pub team_task_initiated: &'static str,

    // Conversation log
    pub conv_log_write_failed: &'static str,
    pub conv_log_open_error: &'static str,

    // Meeting room
    pub meeting_empty: &'static str,

    // Settings view
    pub language_label: &'static str,
    pub theme_label: &'static str,
    pub log_verbosity_label: &'static str,
    pub terminal_app_label: &'static str,
    pub history_size_label: &'static str,
    pub meeting_timeout_label: &'static str,
    pub save_button: &'static str,

    // Auto README
    pub auto_readme_generating: &'static str,

    // Telegram
    pub telegram_connected: &'static str,
    pub telegram_error: &'static str,
    pub telegram_paired: &'static str,
    pub telegram_pairing_code_msg: &'static str,
}

static EN: Messages = Messages {
    agent_ready: "Agent '{}' ready",
    agent_completed: "Agent '{}' completed turn",
    agent_error: "Agent '{}' error: {}",

    task_created: "Task created: {}",
    task_assigned: "Task {} assigned to {}",
    task_completed: "Task {} completed",

    team_planning: "Planning",
    team_executing: "Executing",
    team_synthesizing: "Synthesizing",
    team_completed: "Completed",

    message_routed: "Message: {} → {}: {}",
    broadcast_sent: "Broadcast sent to {} agents",

    unknown_command: "Unknown command: {}",
    usage_send: "Usage: :send <agent> <message>",
    usage_broadcast: "Usage: :broadcast <message>",
    usage_teamtask: "Usage: :teamtask <description>",

    settings_saved: "Settings saved",
    settings_save_error: "Settings save error: {}",
    settings_tab_title: "Settings",
    log_tab_title: "Logs",
    dashboard_title: "Dashboard",
    agent_detail_title: "Agent Detail",
    tasks_title: "Tasks",
    office_title: "Office",

    help_title: "Help",

    command_title: " Command ",
    prompt_title: " Prompt ",
    task_input_title: " New Task ",

    team_task_initiated: "Team task initiated: {}",

    conv_log_write_failed: "Conversation log write failed: {} — disabling log",
    conv_log_open_error: "Cannot open conversation log: {}",

    meeting_empty: "(empty - no active meetings)",

    language_label: "Language",
    theme_label: "Theme",
    log_verbosity_label: "Log Verbosity",
    terminal_app_label: "Terminal App",
    history_size_label: "History Size",
    meeting_timeout_label: "Meeting Timeout",
    save_button: "[Save] (s)",

    auto_readme_generating: "Generating README via lead agent...",

    telegram_connected: "Telegram bridge connected",
    telegram_error: "Telegram error: {}",
    telegram_paired: "Telegram paired with chat {}",
    telegram_pairing_code_msg: "Pairing code: {} — send to your Telegram bot",
};

static TR: Messages = Messages {
    agent_ready: "Ajan '{}' hazır",
    agent_completed: "Ajan '{}' turunu tamamladı",
    agent_error: "Ajan '{}' hata: {}",

    task_created: "Görev oluşturuldu: {}",
    task_assigned: "Görev {} ajana atandı: {}",
    task_completed: "Görev {} tamamlandı",

    team_planning: "Planlama",
    team_executing: "Yürütme",
    team_synthesizing: "Sentezleme",
    team_completed: "Tamamlandı",

    message_routed: "Mesaj: {} → {}: {}",
    broadcast_sent: "{} ajana yayın gönderildi",

    unknown_command: "Bilinmeyen komut: {}",
    usage_send: "Kullanım: :send <ajan> <mesaj>",
    usage_broadcast: "Kullanım: :broadcast <mesaj>",
    usage_teamtask: "Kullanım: :teamtask <açıklama>",

    settings_saved: "Ayarlar kaydedildi",
    settings_save_error: "Ayarlar kaydetme hatası: {}",
    settings_tab_title: "Ayarlar",
    log_tab_title: "Loglar",
    dashboard_title: "Panel",
    agent_detail_title: "Ajan Detay",
    tasks_title: "Görevler",
    office_title: "Ofis",

    help_title: "Yardım",

    command_title: " Komut ",
    prompt_title: " İstem ",
    task_input_title: " Yeni Görev ",

    team_task_initiated: "Takım görevi başlatıldı: {}",

    conv_log_write_failed: "Konuşma logu yazılamadı: {} — log devre dışı",
    conv_log_open_error: "Konuşma logu açılamıyor: {}",

    meeting_empty: "(boş - aktif toplantı yok)",

    language_label: "Dil",
    theme_label: "Tema",
    log_verbosity_label: "Log Detayı",
    terminal_app_label: "Terminal Uyg.",
    history_size_label: "Geçmiş Boyutu",
    meeting_timeout_label: "Toplantı Zaman Aşımı",
    save_button: "[Kaydet] (s)",

    auto_readme_generating: "Lead ajan ile README oluşturuluyor...",

    telegram_connected: "Telegram köprüsü bağlandı",
    telegram_error: "Telegram hatası: {}",
    telegram_paired: "Telegram eşleştirildi, chat {}",
    telegram_pairing_code_msg: "Eşleştirme kodu: {} — Telegram botunuza gönderin",
};

impl Messages {
    pub fn for_lang(lang: &Language) -> &'static Messages {
        match lang {
            Language::En => &EN,
            Language::Tr => &TR,
        }
    }
}

/// Simple format helper: replaces `{}` placeholders left-to-right.
pub fn fmt(template: &str, args: &[&str]) -> String {
    let mut result = template.to_string();
    for arg in args {
        if let Some(pos) = result.find("{}") {
            result.replace_range(pos..pos + 2, arg);
        }
    }
    result
}

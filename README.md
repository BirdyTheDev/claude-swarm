# claude-swarm

Terminal-based multi-agent Claude orchestrator with a professional TUI dashboard. Manage multiple Claude Code instances simultaneously, route messages between agents, distribute tasks, and monitor everything in real-time.

**[Documentation Site](https://BirdyTheDev.github.io/claude-swarm/)** | **[Turkce](#turkce)** | **[English](#english)**

---

<a id="english"></a>

## English

### What is claude-swarm?

claude-swarm is a Rust-based terminal application that orchestrates multiple Claude Code CLI instances as a coordinated agent swarm. Each agent has its own role, skills, and permissions. A central orchestrator manages communication, task assignment, and lifecycle — all visualized in a tmux-quality TUI dashboard.

### Features

| Feature | Description |
|---------|-------------|
| **Multi-Agent Orchestration** | Run multiple Claude Code instances simultaneously with individual roles, models, and skills |
| **Team Tasks (3-Phase)** | `:tt` command triggers Planning → Executing → Synthesizing across all agents |
| **Single Agent Tasks** | `:t` command sends a task directly to the selected agent |
| **Inter-Agent Communication** | Agents exchange messages; Office view shows meetings in real-time |
| **Telegram Integration** | Control your swarm remotely via Telegram bot — send tasks, check status, get notifications |
| **Scheduled Tasks** | `:schedule 09:00 :t dev fix bug` — schedule any command to fire at a specific time (local time) |
| **Performance View** | Process resources (own + claude CLI), aggregate stats (cost/hr, success rate), token usage per agent |
| **Soul System** | `:soul <agent> <personality>` — give each agent a persistent personality/directive |
| **Build Verification** | Auto-verify builds after agent completion, auto-retry on failure (configurable) |
| **7 View Tabs** | Dashboard, Agent Detail, Tasks, Logs, Office, Settings, Performance |
| **3 Themes** | Dark, Light, High Contrast — switch instantly from Settings |
| **i18n (EN/TR)** | All logs and UI strings in English and Turkish |
| **Structured Logs** | Categorized with icons (🤖📋👥💬⚙❌) and verbosity filtering |
| **Multi-line Input** | Enter = new line, Ctrl+Enter = submit, Up/Down = history |
| **Persistent Settings** | `~/.claude-swarm/settings.toml` — language, theme, verbosity, etc. |
| **Broadcast** | `:bc` sends same prompt to all agents simultaneously |
| **Visible Terminals** | `--visible-terminals` opens each agent in a separate Terminal window |

### Preview

```
⬡ my-project-swarm │ Dashboard │ Agent Detail │ Tasks │ Logs │ Office │ Settings │ Performance
┌──────────────────┐┌─────────────────────┐┌──────────────────────────┐
│ Agents           ││ Mini Panels          ││ Focused Output           │
│                  │├─────────────────────┤│                          │
│ ▸ architect ●    ││ [developer] working  ││ 🤖 Agent 'architect' rdy │
│   developer ●    ││  >_ typing...        ││ 📋 Task created: Review  │
│   reviewer  ○    │├─────────────────────┤│ 👥 Team [Planning]: ...  │
│   tester    ◌    ││ [reviewer] idle      ││ 💬 architect → developer │
│                  ││  zzz                 ││ ⚙  Settings saved        │
└──────────────────┘└─────────────────────┘└──────────────────────────┘
 NORMAL │ agents: 4 │ cost: $0.37 │ tasks: 2 pending, 1 active
```

### Why?

- **Parallel work:** Have an architect plan while a developer implements and a reviewer checks code — all at the same time
- **Specialization:** Each agent gets its own system prompt, model, tool permissions, and skill set
- **Coordination:** Agents can send messages to each other, share artifacts, and request work
- **Visibility:** Watch all agents work in real-time from a single terminal window
- **Zero extra cost:** Uses Claude Code CLI under your existing subscription — no API keys needed

### Installation

```bash
# Clone and build
git clone https://github.com/BirdyTheDev/claude-swarm.git
cd claude-swarm
cargo install --path .

# Or directly from source
cargo build --release
./target/release/claude-swarm --help
```

#### Prerequisites

- **Rust** 1.70+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- **Claude Code CLI** installed and authenticated (`claude` must be in your PATH)
- A Claude **Max** or **Pro** subscription

### Quick Start

```bash
# 1. Copy the example config
cp swarm.toml.example swarm.toml

# 2. Edit to your needs (optional)
vim swarm.toml

# 3. Run
claude-swarm --config swarm.toml

# Or run with an initial prompt
claude-swarm --config swarm.toml --prompt "Analyze the project structure"

# Or run with only specific agents
claude-swarm --config swarm.toml --agents architect,developer
```

### Configuration

claude-swarm is configured via a `swarm.toml` file. Each `[[agent]]` block defines an agent in your swarm.

```toml
# Swarm metadata
name = "my-project-swarm"
description = "Multi-agent development team"

# Lead agent (exactly one required)
[[agent]]
name = "architect"
role = "Lead Architect"
system_prompt = "You are a senior software architect."
model = "opus"
skills = ["architecture", "planning", "review"]
allowed_tools = ["Read", "Glob", "Grep"]
permission_mode = "plan"
is_lead = true

# Worker agent
[[agent]]
name = "developer"
role = "Full-Stack Developer"
system_prompt = "You are a skilled developer. Write clean, tested code."
model = "sonnet"
skills = ["coding", "testing", "debugging"]
allowed_tools = ["Read", "Write", "Edit", "Glob", "Grep", "Bash"]
permission_mode = "full-auto"
```

#### Agent Configuration Fields

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Unique agent identifier |
| `role` | No | Human-readable role description |
| `system_prompt` | No | System prompt sent to Claude Code |
| `model` | No | Model to use (`opus`, `sonnet`, `haiku`) |
| `skills` | No | Skills for task-based auto-assignment |
| `allowed_tools` | No | Tools the agent can use |
| `permission_mode` | No | `default`, `plan`, or `full-auto` |
| `max_turns` | No | Maximum conversation turns |
| `max_budget_usd` | No | Budget limit (informational) |
| `is_lead` | No | Exactly one agent must be lead (`true`) |

#### User Settings

User preferences are stored in `~/.claude-swarm/settings.toml` and can be edited from the Settings tab (key `6`):

```toml
language = "en"             # en | tr
theme = "dark"              # dark | light | highcontrast
log_verbosity = "normal"    # minimal | normal | detailed
terminal_app = "Terminal"
input_history_size = 50
meeting_timeout_secs = 10
auto_readme = false         # auto-generate README after task completion
auto_verify = false         # auto-run build verification after agent completion
verify_command = ""         # custom verify command (auto-detected if empty)
max_verify_retries = 3      # max retries on build failure
telegram_enabled = false
telegram_bot_token = ""
telegram_chat_id = ""       # auto-set after pairing
```

### Keybindings

#### Navigation

| Key | Action |
|-----|--------|
| `j` / `Down` | Select next agent |
| `k` / `Up` | Select previous agent |
| `1`-`7` | Switch view tab (Dashboard, Agent Detail, Tasks, Logs, Office, Settings, Performance) |
| `Tab` | Cycle through view tabs |
| `Enter` | Open detail view / Toggle setting value |

#### Agent Interaction

| Key | Action |
|-----|--------|
| `p` | Send prompt to selected agent |
| `:` | Enter command mode |

#### Commands

| Command | Description |
|---------|-------------|
| `:t <desc>` | Send task to selected agent |
| `:tt <desc>` | Team task — all agents collaborate (3-phase) |
| `:task <desc>` | Create a new task (auto-assign) |
| `:bc <msg>` | Broadcast same prompt to all agents |
| `:send <agent> <msg>` | Send inter-agent message |
| `:stop <agent>` | Stop a specific agent |
| `:soul <agent> <text>` | Set agent's soul/personality |
| `:q` / `:quit` | Quit |

#### Telegram Commands

All commands above also work via Telegram. Additional Telegram-only commands:

| Command | Description |
|---------|-------------|
| `:status` | Show all agents and their current status |
| `:cost` | Show token usage and cost per agent |
| `:schedule HH:MM <cmd>` | Schedule a command to fire at local time (24h format) |
| `:schedules` | List all pending scheduled tasks |

**Schedule examples:**
```
:schedule 09:00 :t developer fix the login bug
:schedule 14:30 :tt refactor the auth module
:schedule 19:00 :bc summarize your progress
```

#### Multi-line Input

| Key | Action |
|-----|--------|
| `Enter` | New line (Prompt/Task mode) / Submit (Command mode) |
| `Ctrl+Enter` | Submit (always) |
| `Up/Down` | Move cursor between lines / History (single line) |
| `Ctrl+Up/Down` | History navigate (always) |
| `Home/End` | Start/end of line |

#### Scrolling

| Key | Action |
|-----|--------|
| `Ctrl-d` | Scroll down 10 lines |
| `Ctrl-u` | Scroll up 10 lines |
| `g` | Scroll to top |
| `G` | Scroll to bottom |

#### Settings Tab (6)

| Key | Action |
|-----|--------|
| `j/k` | Navigate settings |
| `Enter` | Toggle/edit value |
| `s` | Save settings |

### Views

1. **Dashboard** — 3-column layout: agent list, mini panels, focused output
2. **Agent Detail** — Full-screen output for the selected agent with metadata
3. **Tasks** — Task lifecycle tracking with status and assignment info
4. **Logs** — Structured logs with category icons and verbosity filtering
5. **Office** — Agent cubicles, meeting room, and message log
6. **Settings** — Interactive settings form (language, theme, Telegram, build verify, etc.)
7. **Performance** — Process resources, aggregate stats, token usage per agent, swarm health

### Structured Log Categories

| Icon | Category | Description |
|------|----------|-------------|
| 🤖 | Agent | Agent ready, completed, working |
| 📋 | Task | Task created, assigned, completed |
| 👥 | Team | Team task phase changes |
| 💬 | Communication | Inter-agent messages, broadcasts |
| ⚙ | System | Settings changes, system events |
| ❌ | Error | Agent errors, failures |

**Verbosity levels:**
- **Minimal** — Only Error + Team phase changes
- **Normal** — Error + Team + Agent ready/completed + Task events
- **Detailed** — Everything including tool use and communication

### Themes

| Theme | Description |
|-------|-------------|
| **Dark** | Default dark theme (cyan accent, dark background) |
| **Light** | Light theme (white background, blue accents) |
| **High Contrast** | Pure black/white with bright colors for accessibility |

### Architecture

```
                    +-------------------+
                    |     main.rs       |
                    |  CLI + TUI Loop   |
                    +--------+----------+
                             |
              +--------------+--------------+
              |                             |
    +---------v---------+         +---------v---------+
    |   Orchestrator    |         |    TUI App        |
    |   (tokio actor)   |         |   (ratatui)       |
    +---+-----+-----+--+         +---------+---------+
        |     |     |                       |
   +----+  +--+  +--+---+           +------+------+
   |       |     |       |          |      |      |
Registry Router Scheduler       Widgets Views  Layout
   |       |     |                 8       7
   +---+---+-----+
       |
  +----v----+
  | AgentPool|
  +----+----+
       |
  +----v---------+----v---------+----v--------+
  | Subprocess 1 | Subprocess 2 | Subprocess 3|
  | (claude CLI) | (claude CLI) | (claude CLI) |
  +--------------+--------------+-------------+
```

### CLI Options

```
claude-swarm [OPTIONS]

Options:
  -c, --config <FILE>       Config file path [default: swarm.toml]
  -p, --prompt <TEXT>       Initial prompt for the lead agent
      --tick-rate <MS>      TUI refresh rate in ms [default: 250]
      --agents <NAMES>      Only spawn these agents (comma-separated)
      --visible-terminals   Open each agent in a separate terminal
      --log-file <FILE>     Log file path [default: claude-swarm.log]
      --log-level <LEVEL>   Log level [default: info]
  -h, --help                Show help
  -V, --version             Show version
```

### Development

```bash
# Run tests
cargo test

# Build in release mode
cargo build --release

# Run with debug logging
cargo run -- --config swarm.toml --log-level debug
```

---

<a id="turkce"></a>

## Turkce

### claude-swarm Nedir?

claude-swarm, birden fazla Claude Code CLI instance'ini koordineli bir ajan surüsü olarak yöneten, Rust ile yazilmis bir terminal uygulamasidir. Her ajanin kendine ait rolü, yetenekleri ve izinleri vardir. Merkezi bir orkestratör iletisimi, görev atamasini ve yasam döngüsünü yönetir — tümü profesyonel bir TUI dashboard'da görsellestirilir.

### Özellikler

| Özellik | Açiklama |
|---------|----------|
| **Çoklu Ajan Orkestrasyonu** | Birden fazla Claude Code örneğini eşanlı çalıştırın |
| **Takım Görevleri (3 Fazlı)** | `:tt` komutu Planlama → Yürütme → Sentezleme fazlarını tetikler |
| **Tekli Ajan Görevi** | `:t` komutu seçili ajana doğrudan görev gönderir |
| **Ajanlar Arası İletişim** | Ajanlar mesaj alışverişi yapar; Ofis görünümü toplantıları gösterir |
| **Telegram Entegrasyonu** | Telegram bot üzerinden uzaktan kontrol — görev gönder, durum sorgula, bildirim al |
| **Zamanlı Görevler** | `:schedule 09:00 :t dev fix bug` — yerel saate göre komut zamanlayın (24 saat formatı) |
| **Performans Görünümü** | Süreç kaynakları (kendi + claude CLI), toplu istatistikler (maliyet/saat, başarı oranı) |
| **Ruh Sistemi** | `:soul <ajan> <kişilik>` — her ajana kalıcı kişilik/yönerge verin |
| **Build Doğrulama** | Ajan tamamlandığında otomatik build kontrolü, hata durumunda otomatik yeniden deneme |
| **7 Görünüm Sekmesi** | Panel, Ajan Detay, Görevler, Loglar, Ofis, Ayarlar, Performans |
| **3 Tema** | Koyu, Açık, Yüksek Kontrast — Ayarlardan anında değiştirin |
| **i18n (EN/TR)** | Tüm loglar ve arayüz metinleri İngilizce ve Türkçe |
| **Yapılandırılmış Loglar** | İkonlarla kategorize (🤖📋👥💬⚙❌) ve detay filtreleme |
| **Çok Satırlı Girdi** | Enter = yeni satır, Ctrl+Enter = gönder, Yukarı/Aşağı = geçmiş |
| **Kalıcı Ayarlar** | `~/.claude-swarm/settings.toml` — dil, tema, detay seviyesi vb. |

### Neden claude-swarm?

- **Paralel çalışma:** Bir mimar planlarken, bir geliştirici uygularken ve bir gözden geçiren kod kontrol edebilir — hepsi aynı anda
- **Uzmanlaşma:** Her ajan kendi sistem prompt'u, modeli, araç izinleri ve yetenek setine sahip olur
- **Koordinasyon:** Ajanlar birbirleriyle mesaj gönderebilir, yapıtlar paylaşabilir ve iş talep edebilir
- **Görünürlük:** Tüm ajanların çalışmasını tek bir terminal penceresinden gerçek zamanlı izleyin
- **Ek maliyet yok:** Mevcut aboneliğiniz altında Claude Code CLI kullanır — API anahtarı gerekmez

### Kurulum

```bash
# Klonla ve kur
git clone https://github.com/BirdyTheDev/claude-swarm.git
cd claude-swarm
cargo install --path .

# Veya doğrudan kaynaktan derle
cargo build --release
./target/release/claude-swarm --help
```

#### Gereksinimler

- **Rust** 1.70+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- **Claude Code CLI** kurulu ve doğrulanmış (PATH'te `claude` komutu olmalı)
- Claude **Max** veya **Pro** aboneliği

### Hızlı Başlangıç

```bash
# 1. Örnek konfigürasyonu kopyala
cp swarm.toml.example swarm.toml

# 2. İhtiyacına göre düzenle (isteğe bağlı)
vim swarm.toml

# 3. Çalıştır
claude-swarm --config swarm.toml

# Başlangıç prompt'u ile çalıştır
claude-swarm --config swarm.toml --prompt "Proje yapısını analiz et"

# Sadece belirli ajanlarla çalıştır
claude-swarm --config swarm.toml --agents architect,developer
```

### Komutlar

| Komut | Açıklama |
|-------|----------|
| `:t <açıklama>` | Seçili ajana görev gönder |
| `:tt <açıklama>` | Takım görevi — tüm ajanlar işbirliği yapar |
| `:task <açıklama>` | Yeni görev oluştur (otomatik atanır) |
| `:bc <mesaj>` | Tüm ajanlara yayın gönder |
| `:send <ajan> <mesaj>` | Ajanlar arası mesaj gönder |
| `:stop <ajan>` | Belirli bir ajanı durdur |
| `:soul <ajan> <metin>` | Ajanın ruhunu/kişiliğini ayarla |
| `:q` / `:quit` | Çık |

#### Telegram Komutları

Yukarıdaki tüm komutlar Telegram üzerinden de çalışır. Ek Telegram komutları:

| Komut | Açıklama |
|-------|----------|
| `:status` | Tüm ajanları ve durumlarını göster |
| `:cost` | Ajan başına token kullanımı ve maliyeti göster |
| `:schedule SS:DD <komut>` | Yerel saate göre komut zamanla (24 saat formatı) |
| `:schedules` | Bekleyen zamanlanmış görevleri listele |

**Zamanlama örnekleri:**
```
:schedule 09:00 :t developer login bugını düzelt
:schedule 14:30 :tt auth modülünü refactor et
:schedule 19:00 :bc ilerlemenizi özetleyin
```

### Görünümler

1. **Panel (1)** — 3 sütunlu düzen: ajan listesi, mini paneller, odaklı çıktı
2. **Ajan Detay (2)** — Seçili ajan için tam ekran çıktı ve meta veriler
3. **Görevler (3)** — Görev yaşam döngüsü takibi
4. **Loglar (4)** — Kategorili yapılandırılmış loglar ve detay filtreleme
5. **Ofis (5)** — Ajan kabinleri, toplantı odası ve mesaj logu
6. **Ayarlar (6)** — Etkileşimli ayarlar formu (dil, tema, Telegram, build doğrulama vb.)
7. **Performans (7)** — Süreç kaynakları, toplu istatistikler, ajan başına token kullanımı, sürü sağlığı

### Geliştirme

```bash
# Testleri çalıştır
cargo test

# Release modunda derle
cargo build --release

# Debug loglama ile çalıştır
cargo run -- --config swarm.toml --log-level debug
```

---

## License / Lisans

MIT

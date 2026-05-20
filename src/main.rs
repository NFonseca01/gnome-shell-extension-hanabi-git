use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Row, Table},
    Terminal,
};
use serde::Deserialize;
use std::{io, process::Command, time::Duration};

#[derive(Deserialize)]
struct AurResult {
    NumVotes: u32,
    Popularity: f64,
    OutOfDate: Option<i64>,
}

#[derive(Deserialize)]
struct AurResponse {
    results: Vec<AurResult>,
}

struct App {
    package_name: String,
    status: String,
    votes: String,
    popularity: String,
    local_version: String,
    contributors: String,
    git_logs: String,
    should_quit: bool,
}

impl App {
    fn new() -> Self {
        let mut app = App {
            package_name: String::from("gnome-shell-extension-hanabi-git"),
            status: String::from("OK"),
            votes: String::from("N/A"),
            popularity: String::from("N/A"),
            local_version: String::from("N/A"),
            contributors: String::from("0"),
            git_logs: String::from("Sin registros de Git locales."),
            should_quit: false,
        };
        app.refresh_data();
        app
    }

    fn refresh_data(&mut self) {
        // 1. Extraer versión (pkgver) directamente desde el archivo .SRCINFO local (Fácil y limpio)
        if let Ok(output) = Command::new("grep").arg("pkgver =").arg("../.SRCINFO").output() {
            let raw = String::from_utf8_lossy(&output.stdout);
            if let Some(line) = raw.lines().next() {
                if let Some(ver) = line.split('=').nth(1) {
                    self.local_version = ver.trim().to_string();
                }
            }
        }
        
        // Agregar el pkgrel secundario si existe para completar el formato (pkgver-pkgrel)
        if let Ok(output) = Command::new("grep").arg("pkgrel =").arg("../.SRCINFO").output() {
            let raw = String::from_utf8_lossy(&output.stdout);
            if let Some(line) = raw.lines().next() {
                if let Some(rel) = line.split('=').nth(1) {
                    if self.local_version != "N/A" {
                        self.local_version = format!("{}-{}", self.local_version, rel.trim());
                    }
                }
            }
        }

        // 2. Contar colaboradores estructurados en las cabeceras del PKGBUILD
        if let Ok(output) = Command::new("grep").arg("-c").arg("# Contributor:").arg("../PKGBUILD").output() {
            self.contributors = String::from_utf8_lossy(&output.stdout).trim().to_string();
        }

        // 3. Extraer telemetría real del historial de Git Staging (Últimos 3 logs de tu repositorio local)
        if let Ok(output) = Command::new("git").arg("-C").arg("..").arg("log").arg("-3").arg("--format= ⚡ %cd | %s").arg("--date=short").output() {
            let logs = String::from_utf8_lossy(&output.stdout);
            if !logs.trim().is_empty() {
                self.git_logs = logs.to_string();
            }
        }

        // 4. Consumir la API oficial RPC de Arch Linux AUR
        let url = format!("https://aur.archlinux.org/rpc/?v=5&type=info&arg[]={}", self.package_name);
        if let Ok(res) = reqwest::blocking::get(&url) {
            if let Ok(aur_data) = res.json::<AurResponse>() {
                if let Some(pkg) = aur_data.results.first() {
                    self.votes = pkg.NumVotes.to_string();
                    self.popularity = format!("{:.2}%", pkg.Popularity);
                    self.status = if pkg.OutOfDate.is_some() {
                        String::from("🚨 OUT-OF-DATE")
                    } else {
                        String::from("✅ OPERATIVO")
                    };
                }
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    while !app.should_quit {
        terminal.draw(|f| ui(f, &app))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
                    KeyCode::Char('r') => app.refresh_data(),
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    Ok(())
}

fn ui(f: &mut ratatui::Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(10), Constraint::Length(3)])
        .split(f.size());

    // Barra Superior de Estado Global
    let title = Paragraph::new(format!(" 📊 AUR TELEMETRY MONITOR | Gestión de Activos de Software"))
        .block(Block::default().borders(Borders::ALL).title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)));
    f.render_widget(title, chunks[0]);

    // Distribución del panel central equilibrada para evitar truncado de nombres largos (45% - 55%)
    let middle_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(chunks[1]);

    // Panel Izquierdo: Métricas del Mercado Extraídas en Tiempo Real
    let status_color = if app.status.contains("🚨") { Color::Red } else { Color::Green };
    let rows = vec![
        Row::new(vec!["Identificador:", &app.package_name]),
        Row::new(vec!["Estado en Web:", &app.status]).style(Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
        Row::new(vec!["Versión Local (.SRCINFO):", &app.local_version]).style(Style::default().fg(Color::Yellow)),
        Row::new(vec!["Votos Totales:", &app.votes]),
        Row::new(vec!["Popularidad de Uso:", &app.popularity]),
        Row::new(vec!["Colaboradores Contados:", &app.contributors]),
    ];

    let table = Table::new(rows, [Constraint::Length(28), Constraint::Length(50)])
        .block(Block::default().borders(Borders::ALL).title(" Inteligencia y Cuota de Mercado "));
    f.render_widget(table, middle_chunks[0]);

    // Panel Derecho: Trazabilidad Real del Historial de Git local
    let git_logs = Paragraph::new(app.git_logs.as_str())
        .block(Block::default().borders(Borders::ALL).title(" Historial de Cumplimiento y Git Staging (Real) "));
    f.render_widget(git_logs, middle_chunks[1]);

    // Barra Inferior de Control de la Consola
    let footer = Paragraph::new(" [Q] Cerrar Monitor  |  [R] Forzar Refresco de API y Archivos Locales")
        .block(Block::default().borders(Borders::ALL).style(Style::default().fg(Color::DarkGray)));
    f.render_widget(footer, chunks[2]);
}

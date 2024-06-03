use std::io::stdout;
use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use crossterm::{event, ExecutableCommand};
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::style::Stylize;
use ratatui::Terminal;
use ratatui::widgets::Paragraph;


/// Simple program to add dev notes
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Varargs{
    ///The command to be run
    #[command(subcommand)]
    mode:CommandMode
}

#[derive(Subcommand,Debug,Clone)]
enum CommandMode{
    /// Use a project to add the notes to
    Use{
        /// The project name to be appended
        project:String
    },
    ///Add Dev note to selected project
    Add{
        ///Name of the name
        name:String,

        /// Optional: Date of note
        date:Option<String>
    },
    ///Delete note from project
    Delete{
        /// Guid prefix of the note to delete
        id:String
    },
    /// List notes for current project
    List,
    /// List selectable Projects
    Projects,
    /// Edit note
    Edit{
        /// id of the note
        guid:String
    }
}


fn main()->Result<()> {
    let args=Varargs::parse();
    // stdout().execute(EnterAlternateScreen)?;
    // enable_raw_mode()?;
    // let mut terminal=Terminal::new(CrosstermBackend::new(stdout()))?;
    // terminal.clear()?;
    //
    // 'main_loop: loop {
    //     terminal.draw(|frame|{
    //         let area =frame.size();
    //         frame.render_widget(Paragraph::new("HEEEY BITTTCH").white().on_black(),area);
    //     })?;
    //
    //     if event::poll(std::time::Duration::from_millis(16))?{
    //         if let event::Event::Key(key)=event::read()?{
    //             if key.kind==KeyEventKind::Press && key.modifiers==KeyModifiers::CONTROL && key.code==KeyCode::Char('q'){
    //                 break 'main_loop;
    //             }
    //         }
    //
    //     }
    // }

    // stdout().execute(LeaveAlternateScreen)?;
    // disable_raw_mode()?;
    Ok(())
}

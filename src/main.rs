mod db;
mod dir;
mod types;
mod util;

use crate::db::DB;
use crate::types::Timestamp;
use crate::util::{fzf_helper, get_current_time, get_db};
use anyhow::{anyhow, Context, Result};
use clap::arg_enum;
use std::env;
use std::path::Path;
use structopt::StructOpt;

// TODO: use structopt to parse env variables: <https://github.com/TeXitoi/structopt/blob/master/examples/env.rs>

arg_enum! {
    #[allow(non_camel_case_types)]
    #[derive(Debug)]
    enum Shell {
        bash,
        fish,
        zsh,
    }
}

#[derive(Debug, StructOpt)]
#[structopt(about = "A cd command that learns your habits")]
enum Zoxide {
    #[structopt(about = "Add a new directory or increment its rank")]
    Add { path: Option<String> },

    #[structopt(about = "Prints shell configuration")]
    Init {
        #[structopt(possible_values = &Shell::variants(), case_insensitive = true)]
        shell: Shell,
    },

    #[structopt(about = "Search for a directory")]
    Query {
        keywords: Vec<String>,
        #[structopt(short, long, help = "Opens an interactive selection menu using fzf")]
        interactive: bool,
    },

    #[structopt(about = "Remove a directory")]
    Remove { path: String },
}

fn zoxide_query(db: &mut DB, mut keywords: Vec<String>, now: Timestamp) -> Option<String> {
    if let [path] = keywords.as_slice() {
        if Path::new(path).is_dir() {
            return Some(path.to_owned());
        }
    }

    for keyword in &mut keywords {
        keyword.make_ascii_lowercase();
    }

    if let Some(dir) = db.query(&keywords, now) {
        return Some(dir.path);
    }

    None
}

fn zoxide_query_interactive(
    db: &mut DB,
    keywords: Vec<String>,
    now: Timestamp,
) -> Result<Option<String>> {
    let dirs = db.query_all(keywords);
    fzf_helper(now, dirs)
}

pub fn main() -> Result<()> {
    let opt = Zoxide::from_args();
    match opt {
        Zoxide::Add { path: path_opt } => {
            let mut db = get_db()?;
            let now = get_current_time()?;

            match path_opt {
                Some(path) => db.add(path, now),
                None => {
                    let current_dir = env::current_dir()
                        .with_context(|| anyhow!("unable to fetch current directory"))?;
                    db.add(current_dir, now)
                }
            }?;

            db.save()?;
        }
        Zoxide::Init { shell } => {
            match shell {
                Shell::bash => {
                    println!("{}", INIT_BASH);
                    println!("{}", INIT_BASH_ALIAS);
                }
                Shell::fish => {
                    println!("{}", INIT_FISH);
                    println!("{}", INIT_FISH_ALIAS);
                }
                Shell::zsh => {
                    println!("{}", INIT_ZSH);
                    println!("{}", INIT_ZSH_ALIAS);
                }
            };
        }
        Zoxide::Query {
            keywords,
            interactive,
        } => {
            let mut db = get_db()?;
            let now = get_current_time()?;

            let path_opt = if interactive {
                zoxide_query_interactive(&mut db, keywords, now)?
            } else {
                zoxide_query(&mut db, keywords, now)
            };

            if let Some(path) = path_opt {
                println!("query: {}", path.trim());
            }
        }
        Zoxide::Remove { path } => {
            let mut db = get_db()?;
            db.remove(path)?;
            db.save()?;
        }
    };

    Ok(())
}

const INIT_BASH: &str = r#"
_zoxide_precmd() {
    zoxide add
}

case "$PROMPT_COMMAND" in
	*_zoxide_precmd*) ;;
	*) PROMPT_COMMAND="_zoxide_precmd${PROMPT_COMMAND:+;$PROMPT_COMMAND}" ;;
esac

z() {
    if [ $# -ne 0 ]; then
        _Z_RESULT=$(zoxide query "$@")
        case $_Z_RESULT in
            "query: "*)
                cd "${_Z_RESULT:7}"
                ;;
            *)
                echo -n "${_Z_RESULT}"
                ;;
        esac
    else
        cd "${HOME}"
    fi
}
"#;

const INIT_BASH_ALIAS: &str = r#"
alias zi="z -i"

alias za="zoxide add"
alias zq="zoxide query"
alias zr="zoxide remove"
"#;

const INIT_FISH: &str = r#"
function _zoxide_precmd --on-event fish_prompt
    zoxide add
end

function z
    if test (count $argv) -gt 0
        set _Z_RESULT (zoxide query $argv)
        switch "$_Z_RESULT"
            case 'query: *'
                cd (string sub -s 8 -- "$_Z_RESULT")
                commandline -f repaint
            case '*'
                echo -n "$_Z_RESULT"
        end
    else
        cd "$HOME"
        commandline -f repaint
    end
end
"#;

const INIT_FISH_ALIAS: &str = r#"
abbr -a zi "z -i"
abbr -a za "zoxide add"
abbr -a zq "zoxide query"
abbr -a zr "zoxide remove"
"#;

const INIT_ZSH: &str = r#"
_zoxide_precmd() {
    zoxide add
}

[[ -n "${precmd_functions[(r)_zoxide_precmd]}" ]] || {
    precmd_functions+=(_zoxide_precmd)
}

z() {
    if [ $# -ne 0 ]; then
        _Z_RESULT=$(zoxide query "$@")
        case $_Z_RESULT in
            "query: "*)
                cd "${_Z_RESULT:7}"
                ;;
            *)
                echo -n "${_Z_RESULT}"
                ;;
        esac
    else
        cd "${HOME}"
    fi
}
"#;

const INIT_ZSH_ALIAS: &str = r#"
alias zi="z -i"

alias za="zoxide add"
alias zq="zoxide query"
alias zr="zoxide remove"
"#;

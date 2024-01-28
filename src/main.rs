use rcon_client::{AuthRequest, RCONClient, RCONConfig, RCONError, RCONRequest};
use std::collections::HashSet;
use std::env;
use std::io;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::fs;
use regex::Regex;

/// Sends an rcon command to palworld.
fn send_rcon_command(port: u16, password: &String, command: String) -> Result<String, RCONError> {
    let mut rcon = RCONClient::new(RCONConfig {
        url: format!("localhost:{}", port),
        write_timeout: None,
        read_timeout: None,
    })?;

    let auth_result = rcon.auth(AuthRequest {
        // lol?
        id: 1,
        // always 3
        request_type: 3,
        // &string.to_string() <- huh?
        password: password.to_string(),
    })?;
    // println!("[<==] Auth: id: {} / type: {}", auth_result.id, auth_result.response_type);
    assert!(auth_result.is_success());

    println!("[==>] Sending \"{}\"", command);
    let result = rcon.execute(RCONRequest {
        id: 0,           // idk
        request_type: 2, // always 2.
        body: command,
    })?;

    // println!("[<==] id: {} / type: {}", result.id, result.response_type);
    println!("[<==] {}", result.body);

    Ok(result.body)
}

// let file_path = Path::new(“path_to_file”);

fn main() -> io::Result<()> {
    // I am not a rust programer, help me pls
    let env_vars_to_print: HashSet<&str> = HashSet::from([
        "GAME_PORT",
        "PLAYER_COUNT",
        "SHOW_IN_COMMUNITY_BROWSER",
        "PUBLIC_IP",
        "PUBLIC_PORT",
        "SERVER_NAME",
        "SERVER_PASSWORD",
        "ADMIN_PASSWORD",
        "QUERY_PORT",
        "MULTITHREADING",
        "RCON_PORT",
        "RCON_ENABLED",
    ]);

    // print env variables for debug purposes.
    for (key, value) in env::vars() {
        if env_vars_to_print.contains(key.as_str()) {
            println!("[Env] {}={}", key, value);
        }
    }

    // let exec_path = fs::canonicalize("./")?;
    // println!("running from dir {}", exec_path.into_os_string().into_string().unwrap());
    // let command_line_arguments: Vec<String> = env::args().collect();

    // execute palserver
    let mut palworld_bin = Command::new("./PalServer.sh");

    // forward our stdio to this process' stdio, for pterodactyl.
    // stdin does absolutely nothing, which is why this program exists.
    palworld_bin.stdout(Stdio::inherit());

    // Section: arguments
    // admin_password should always exist for this program to work.
    let admin_password = match std::env::var("ADMIN_PASSWORD") {
        Ok(result) => result,
        Err(_) => "password".to_string(),
    };

    // also rcon_port must be set.
    let rcon_port : u16 = match std::env::var("RCON_PORT") {
        Ok(result) => result.parse().unwrap_or(25575),
        Err(_) => 25575,
    };

    {
        // admin password (used for rcon)
        palworld_bin.arg(format!("-adminpassword={}", admin_password));

        // server name
        match std::env::var("SERVER_NAME") {
            Ok(value) => {
                if !value.is_empty() {
                    palworld_bin.arg(format!("-servername={}", value));
                }
            }
            Err(_) => {}
        };

        // server password
        match std::env::var("SERVER_PASSWORD") {
            Ok(value) => {
                if !value.is_empty() {
                    palworld_bin.arg(format!("-serverpassword={}", value));
                }
            }
            Err(_) => {}
        };

        // playercount
        match std::env::var("PLAYER_COUNT") {
            Ok(value) => {
                if !value.is_empty() {
                    palworld_bin.arg(format!("-players={}", value));
                }
            }
            Err(_) => {}
        };

        // show server in community browser
        match std::env::var("SHOW_IN_COMMUNITY_BROWSER") {
            Ok(value) => {
                if value == "true" {
                    palworld_bin.arg("EpicApp=PalServer");
                }
            }
            Err(_) => {}
        };

        // port to host the server, default 8211.
        match std::env::var("GAME_PORT") {
            Ok(port) => {
                if !port.is_empty() {
                    palworld_bin.arg(format!("-port={}", port));
                }
            }
            Err(_) => {}
        };

        // "Query Port" / Don't ask me
        match std::env::var("QUERY_PORT") {
            Ok(value) => {
                if !value.is_empty() {
                    palworld_bin.arg(format!("-queryport={}", value));
                }
            }
            Err(_) => {}
        };

        // "Public IP" / probably reverse proxy related
        match std::env::var("PUBLIC_IP") {
            Ok(ip) => {
                if !ip.is_empty() {
                    palworld_bin.arg(format!("-publicip={}", ip));
                }
            }
            Err(_) => {}
        };

        // "Public Port" / probably reverse proxy related
        match std::env::var("PUBLIC_PORT") {
            Ok(value) => {
                if !value.is_empty() {
                    palworld_bin.arg(format!("-publicport={}", value));
                }
            }
            Err(_) => {}
        };

        // Enable Multithreading
        match std::env::var("MULTITHREADING") {
            Ok(value) => {
                if value == "true" {
                    palworld_bin.args([
                        "-useperfthreads",
                        "-NoAsyncLoadingThread",
                        "-UseMultithreadForDS",
                    ]);
                }
            }
            Err(_) => {}
        };
    }

    let mut world_settings = fs::read_to_string("./Pal/Saved/Config/LinuxServer/PalWorldSettings.ini").unwrap();
    
    // probably first run.
    if world_settings.replace(" ", "").is_empty() {
        println!("[!] PalWorldSettings is empty!");
        // TODO: run and kill palworld for like 10 seconds and copy default settings
        // palworld_bin.spawn()
    }

    // always enable rcon
    let regex_enable = Regex::new(r"RCONEnabled=[a-zA-Z]*").unwrap();
    world_settings = regex_enable.replace_all(&world_settings, "RCONEnabled=True").to_string();

    // set rcon port
    let regex_rcon_port = Regex::new(r"RCONPort=[0-9]*").unwrap();
    world_settings = regex_rcon_port.replace_all(&world_settings, format!("RCONPort={}", rcon_port)).to_string();

    fs::write("./Pal/Saved/Config/LinuxServer/PalWorldSettings.ini", world_settings)?;

    // create our rcon client stdin thread.
    thread::spawn(move || -> Result<(), std::io::Error> {
        loop {
            let mut buffer = String::new();
            let stdin = io::stdin(); // We get `Stdin` here.
            stdin.read_line(&mut buffer)?;

            match send_rcon_command(rcon_port, &admin_password, buffer.replace("\n", "")) {
                Ok(result) => result,
                Err(_) => continue,
            };
        }
    });

    print!("Executing with args");
    for arg in palworld_bin.get_args() {
        print!(" {:?}", arg.to_str());
    }
    println!();

    palworld_bin
        .spawn()
        .expect("[!] Palworld Failed to execute.")
        .wait()?;

    Ok(())
}

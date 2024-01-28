use std::io;
use std::env;
use std::collections::HashSet;
use std::process::{Command, Stdio, ExitStatus};
use rcon_client::{AuthRequest, RCONClient, RCONConfig, RCONError, RCONRequest};
use std::thread;

/// Sends an rcon command to palworld.
fn send_rcon_command(port: u16, password: &String, command : String) -> Result<String, RCONError>  {
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
        password: password.to_string()
    })?;
    // println!("[<==] Auth: id: {} / type: {}", auth_result.id, auth_result.response_type);
    assert!(auth_result.is_success());

    println!("[==>] sending {}", command);
    let result = rcon.execute(RCONRequest{
        id: 0, // idk 
        request_type: 2, // always 2.
        body: command
    })?;

    // println!("[<==] id: {} / type: {}", result.id, result.response_type);
    println!("[<==] {}", result.body);
    
    Ok(result.body)
} 


fn main() -> io::Result<()> {
    // I am not a rust programer, help me pls
    let env_vars_to_print : HashSet<&str> = HashSet::from([
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
        "RCON_ENABLED"
    ]);

    // print env variables for debug purposes.
    for (key, value) in env::vars() {
        if env_vars_to_print.contains(key.as_str()) {
            println!("env var {key}: {value}");
        }
    }

    // let exec_path = fs::canonicalize("./")?;
    // println!("running from dir {}", exec_path.into_os_string().into_string().unwrap());
    // let command_line_arguments: Vec<String> = env::args().collect();

    // execute palserver
    let mut palworld_bin  = Command::new("./PalServer.sh");

    // forward our stdio to this process' stdio, for pterodactyl.
    // stdin does absolutely nothing, which is why this program exists.
    palworld_bin.stdout(Stdio::inherit());

    // admin_password should always exist for this program to work.
    let admin_password = match std::env::var("ADMIN_PASSWORD") {
        Ok(result) => result,
        Err(_) => "password".to_string()
    };
    // add to program arguments
    palworld_bin.arg(format!("-adminpassword={}", admin_password));

    // execute palworld, and wait exit when it exits.
    let palserver_thread = thread::spawn(move || -> Result<ExitStatus, std::io::Error> {
        Ok(palworld_bin.spawn().expect("[!] Palworld Failed to execute.").wait()?)
    });

    // create our rcon client
    thread::spawn(move || -> Result<(), std::io::Error> {
        loop {
            let mut buffer = String::new();
            let stdin = io::stdin(); // We get `Stdin` here.
            stdin.read_line(&mut buffer)?;

            match send_rcon_command(25575, &admin_password, buffer.replace("\n", "")) {
                Ok(result) => result,
                Err(_) => continue
            };
        }
    });

    palserver_thread.join().unwrap()?;

    Ok(())
}

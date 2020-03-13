#[macro_use]
extern crate prettytable;

use byte_unit::{Byte, ByteUnit};
use clap::{App, Arg, SubCommand};
use prettytable::{color, Attr, Cell, Row, Table};
use std::env;
use std::io::Write;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use sysinfo::{DiskExt, SystemExt};

struct HardwareRequirement {
    cpu_cores: u8,
    memory: Byte,
    storage_space: Byte,
    ports: Vec<u16>,
}

fn get_color(actual: u64, required: u64, expected: u64) -> color::Color {
    if actual < required {
        return color::RED;
    }
    if actual < expected {
        return color::YELLOW;
    }
    return color::GREEN;
}

fn is_port_available(port: &u16) -> bool {
    match TcpListener::bind(("127.0.0.1", *port)) {
        Ok(_) => true,
        Err(_) => false,
    }
}

fn check_hardware_requirement(force: &bool) {
    let required_requirement = HardwareRequirement {
        cpu_cores: 2,
        memory: Byte::from_unit(4.0, ByteUnit::GiB).unwrap(),
        storage_space: Byte::from_unit(40.0, ByteUnit::GiB).unwrap(),
        // TODO: check 80 8800
        ports: vec![8811],
    };
    let expeceted_requirement = HardwareRequirement {
        cpu_cores: 4,
        memory: Byte::from_unit(8.0, ByteUnit::GiB).unwrap(),
        storage_space: Byte::from_unit(100.0, ByteUnit::GiB).unwrap(),
        ports: vec![],
    };

    let mut system = sysinfo::System::new_all();
    system.refresh_all();

    let mut table = Table::new();
    table.add_row(row!["", "Required", "Expected", "Actual"]);

    let actual_cpu_cores = system.get_processors().len() as u8;
    table.add_row(Row::new(vec![
        Cell::new("cpu cores"),
        Cell::new(&required_requirement.cpu_cores.to_string()),
        Cell::new(&expeceted_requirement.cpu_cores.to_string()),
        Cell::new(&actual_cpu_cores.to_string())
            .with_style(Attr::ForegroundColor(get_color(
                actual_cpu_cores as u64,
                required_requirement.cpu_cores as u64,
                expeceted_requirement.cpu_cores as u64,
            )))
            .with_style(Attr::Bold),
    ]));

    let actual_memory = Byte::from_unit(system.get_total_memory() as f64, ByteUnit::KiB).unwrap();
    table.add_row(Row::new(vec![
        Cell::new("memory"),
        Cell::new(
            &required_requirement
                .memory
                .get_appropriate_unit(true)
                .to_string(),
        ),
        Cell::new(
            &expeceted_requirement
                .memory
                .get_appropriate_unit(true)
                .to_string(),
        ),
        Cell::new(&actual_memory.get_appropriate_unit(true).to_string())
            .with_style(Attr::ForegroundColor(get_color(
                actual_memory.get_bytes() as u64,
                required_requirement.memory.get_bytes() as u64,
                expeceted_requirement.memory.get_bytes() as u64,
            )))
            .with_style(Attr::Bold),
    ]));

    let remained_storage_space = Byte::from_bytes(
        system
            .get_disks()
            .iter()
            .fold(0, |sum, disk| sum + disk.get_available_space() as u128),
    );
    table.add_row(Row::new(vec![
        Cell::new("storage space"),
        Cell::new(
            &required_requirement
                .storage_space
                .get_appropriate_unit(true)
                .to_string(),
        ),
        Cell::new(
            &expeceted_requirement
                .storage_space
                .get_appropriate_unit(true)
                .to_string(),
        ),
        Cell::new(
            &remained_storage_space
                .get_appropriate_unit(true)
                .to_string(),
        )
        .with_style(Attr::ForegroundColor(get_color(
            remained_storage_space.get_bytes() as u64,
            required_requirement.storage_space.get_bytes() as u64,
            expeceted_requirement.storage_space.get_bytes() as u64,
        )))
        .with_style(Attr::Bold),
    ]));

    let unavailable_ports = &required_requirement
        .ports
        .clone()
        .into_iter()
        .filter(|p| !is_port_available(p))
        .collect::<Vec<u16>>();
    let port_result_cell = if unavailable_ports.len() == 0 {
        Cell::new("Ok").with_style(Attr::ForegroundColor(color::GREEN))
    } else {
        Cell::new(
            &unavailable_ports
                .clone()
                .into_iter()
                .map(|p| p.to_string())
                .collect::<Vec<String>>()
                .join(", "),
        )
        .with_style(Attr::ForegroundColor(color::RED))
    };
    table.add_row(Row::new(vec![
        Cell::new("ports"),
        Cell::new(
            &required_requirement
                .ports
                .into_iter()
                .map(|p| p.to_string())
                .collect::<Vec<String>>()
                .join(", "),
        ),
        Cell::new("-"),
        port_result_cell.with_style(Attr::Bold),
    ]));

    table.printstd();

    if *force {
        return;
    }

    if actual_cpu_cores < required_requirement.cpu_cores {
        panic!("CPU cores not enough.");
    }
    if actual_memory < required_requirement.memory {
        panic!("Memory not enough.")
    }
    if remained_storage_space < required_requirement.storage_space {
        panic!("Storage space not enough.");
    }
    if unavailable_ports.len() > 0 {
        panic!("Some ports are not available.");
    }
}

fn check_docker() {
    let docker_status = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(&["/C", "docker info"])
            .stdout(Stdio::null())
            .status()
            .expect("docker is not running")
            .success()
    } else {
        Command::new("sh")
            .arg("-c")
            .arg("docker info")
            .stdout(Stdio::null())
            .status()
            .expect("docker is not running")
            .success()
    };
    if !docker_status {
        panic!("docker is not running")
    } else {
        println!("docker is running")
    }
    let docker_compose_status = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(&["/C", "docker-compose version"])
            .stdout(Stdio::null())
            .status()
    } else {
        Command::new("sh")
            .arg("-c")
            .arg("docker-compose version")
            .stdout(Stdio::null())
            .status()
    };
    if !docker_compose_status
        .expect("docker-compose was not installed")
        .success()
    {
        panic!("docker-compose was not installed")
    } else {
        println!("docker-compose was installed")
    }
}

fn start_from_source(source_dir: &PathBuf, force: &bool) {
    let docker_compose_file = source_dir
        .join("packages/server/docker-compose.yml")
        .to_str()
        .expect("failed to get docker-compose.yml")
        .to_owned();
    let docker_compose_arg = format!("docker-compose -p tower -f {} up -d", &docker_compose_file);
    let containers_command = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(&["/C", &docker_compose_arg])
            .status()
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(&docker_compose_arg)
            .status()
    };
    if !containers_command
        .expect("failed to start tower containers")
        .success()
    {
        panic!("failed to start tower containers")
    }
    let source_dir_str = source_dir.to_str().unwrap();
    let yarn_arg = format!(
        "cd {} && yarn && yarn lerna run prepublish",
        &source_dir_str
    );
    let yarn_command = if cfg!(target_os = "windows") {
        Command::new("cmd").args(&["/C", &yarn_arg]).status()
    } else {
        Command::new("sh").arg("-c").arg(&yarn_arg).status()
    };
    if !yarn_command
        .expect("failed to build tower from source code")
        .success()
    {
        panic!("failed to build tower from source code")
    }

    // cd packages/server && yarn prisma deploy
    let server_dir = source_dir.join("packages/server");
    let setup_script = server_dir
        .join("scripts/setup.js")
        .to_str()
        .expect("failed to get setup.js")
        .to_owned();
    let reset_arg = if *force {
        "&& yarn prisma reset -f"
    } else {
        ""
    };
    let setup_args = &[
        // cd to server dir
        "cd",
        server_dir.to_str().unwrap(),
        // deploy
        "&& yarn prisma deploy",
        // reset data when force flag provided
        reset_arg,
        // execute setup script
        "&& node",
        &setup_script,
    ]
    .join(" ");
    let setup_command = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .env("PRISMA_PORT", "8811")
            .arg("/C")
            .arg(setup_args)
            .status()
    } else {
        Command::new("sh")
            .env("PRISMA_PORT", "8811")
            .arg("-c")
            .arg(setup_args)
            .status()
    };
    if !setup_command.expect("failed to run setup script").success() {
        panic!("failed to run setup script")
    }
}

fn check_images() -> bool {
    let images = [
        "tower:0.2.3",
        "prismagraphql/prisma:1.34",
        "postgres:10.3",
        "openresty/openresty:alpine",
    ];
    for image in images.iter() {
        let command = format!("docker inspect --type=image {}", &image);
        let status = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(&["/C", &command])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
        } else {
            Command::new("sh")
                .arg("-c")
                .arg(&command)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
        };
        let image_exists = match status {
            Ok(s) => s.success(),
            Err(_) => false,
        };
        if !image_exists {
            println!("{} image is missing", &image);
            return false;
        }
    }
    println!("all image exists");
    true
}

fn load_images(tar_dir: &PathBuf) {
    let docker_arg = format!("docker load --input {}", tar_dir.to_str().unwrap());
    let command = if cfg!(target_os = "windows") {
        Command::new("cmd").args(&["/C", &docker_arg]).status()
    } else {
        Command::new("sh").arg("-c").arg(&docker_arg).status()
    };
    if !command.expect("failed to load docker images").success() {
        panic!("failed to load docker images")
    }
}

fn start_from_image() {
    let docker_compose_arg = "docker-compose -p tower -f - up -d";
    let mut child = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(&["/C", &docker_compose_arg])
            .stdin(Stdio::piped())
            .spawn()
            .expect("failed to start tower containers")
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(&docker_compose_arg)
            .stdin(Stdio::piped())
            .spawn()
            .expect("failed to start tower containers")
    };
    {
        let child_stdin = child.stdin.as_mut().expect("failed to get stdin handler");
        child_stdin
            .write_all(
                b"
version: '3'
services:
  prisma:
    image: prismagraphql/prisma:1.34
    restart: always
    depends_on:
      - 'postgres'
    ports:
      - '8811:8811'
    environment:
      PRISMA_CONFIG: |
        port: 8811
        databases:
          default:
            connector: postgres
            host: postgres
            port: 5432
            user: prisma
            password: prisma
            rawAccess: true
  postgres:
    image: postgres:10.3
    restart: always
    environment:
      POSTGRES_USER: prisma
      POSTGRES_PASSWORD: prisma
    volumes:
      - postgres:/var/lib/postgresql/data
  openresty:
    image: openresty/openresty:alpine
    restart: always
    ports:
      - '80:80'
    environment:
      - NGINX_PORT=80
    volumes:
      - ../server/config/nginx:/etc/nginx/conf.d
      - ../ui/build:/www/tower
  server:
    image: tower:0.2.3
    restart: always
    depends_on:
      - 'prisma'
    ports:
      - '8800:8800'
volumes:
  postgres: ~
",
            )
            .expect("failed to start tower containers");
    }
    if !child
        .wait()
        .expect("failed to start tower containers")
        .success()
    {
        panic!("failed to start tower containers")
    }
}

fn shut_down() {
    let docker_compose_arg = "docker-compose -p tower down";
    let containers_command = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(&["/C", &docker_compose_arg])
            .status()
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(&docker_compose_arg)
            .status()
    };
    if !containers_command
        .expect("failed to shut down tower containers")
        .success()
    {
        panic!("failed to shut down tower containers")
    }
}

fn main() {
    let matches = App::new("Tower Installer")
        .version("0.1.0")
        .subcommand(SubCommand::with_name("down").about("Shut down tower serivce."))
        .subcommand(
            SubCommand::with_name("deploy").about("Deploy tower service")
                .arg(
                    Arg::with_name("source_dir")
                        .long("from-source")
                        .help("Install tower from source code, please provide the directory of tower source code.")
                        .takes_value(true)
                    )
                .arg(
                    Arg::with_name("tar_dir")
                        .long("from-tar")
                        .help("Load tower images from target directory.")
                        .takes_value(true)
                )
                .arg(Arg::with_name("force").long("force").help("Reset data and force deploy a new tower service")
            )
        )
        .get_matches();

    match matches.subcommand() {
        ("deploy", Some(sub_matches)) => {
            let force = &sub_matches.is_present("force");
            println!("> checking hardware requirements...");
            check_hardware_requirement(force);
            println!("> checking docker and docker-compose...");
            check_docker();
            check_images();
            println!("> starting tower containers...");
            if let Some(source_dir_value) = sub_matches.value_of("source_dir") {
                let mut source_dir = PathBuf::from(source_dir_value);
                if !source_dir.is_absolute() {
                    source_dir = env::current_dir()
                        .expect("failed to get current directory")
                        .join(source_dir)
                        .canonicalize()
                        .expect("failed to canoicalize source directory");
                }
                start_from_source(&source_dir, force);
                return;
            }
            if let Some(tar_dir_value) = sub_matches.value_of("tar_dir") {
                let mut tar_dir = PathBuf::from(tar_dir_value);
                if !tar_dir.is_absolute() {
                    tar_dir = env::current_dir()
                        .expect("failed to get current directory")
                        .join(tar_dir)
                        .canonicalize()
                        .expect("failed to canoicalize source directory");
                    load_images(&tar_dir)
                }
            }
            start_from_image()
        }
        ("down", Some(_)) => {
            shut_down();
        }
        _ => {}
    };
}

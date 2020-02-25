#[macro_use]
extern crate prettytable;

use byte_unit::{Byte, ByteUnit};
use clap::{App, Arg, SubCommand};
use prettytable::{color, Attr, Cell, Row, Table};
use std::env;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use sysinfo::{DiskExt, SystemExt};

struct HardwareRequirement {
    cpu_cores: u8,
    memory: Byte,
    storage_space: Byte,
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

fn check_hardware_requirement() {
    let required_requirement = HardwareRequirement {
        cpu_cores: 2,
        memory: Byte::from_unit(4.0, ByteUnit::GiB).unwrap(),
        storage_space: Byte::from_unit(40.0, ByteUnit::GiB).unwrap(),
    };
    let expeceted_requirement = HardwareRequirement {
        cpu_cores: 4,
        memory: Byte::from_unit(8.0, ByteUnit::GiB).unwrap(),
        storage_space: Byte::from_unit(100.0, ByteUnit::GiB).unwrap(),
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

    table.printstd();

    if actual_cpu_cores < required_requirement.cpu_cores {
        panic!("CPU cores not enough.");
    }
    if actual_memory < required_requirement.memory {
        panic!("Memory not enough.")
    }
    if remained_storage_space < required_requirement.storage_space {
        panic!("Storage space not enough.");
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

fn start_from_source(source_dir: &PathBuf, force: bool) {
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
    let reset_arg = if force { "&& yarn prisma reset -f" } else { "" };
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

fn main() {
    let matches = App::new("Tower Installer")
        .version("0.1.0")
        .subcommand(SubCommand::with_name("deploy").arg(Arg::with_name("source_dir").long("from-source").help(
            "Install tower from source code, please provide the directory of tower source code.",
        ).takes_value(true))
        .arg(Arg::with_name("force").long("force").help("Reset data and force deploy a new tower service")))
        .get_matches();

    match matches.subcommand() {
        ("deploy", Some(sub_matches)) => {
            println!("> checking hardware requirements...");
            check_hardware_requirement();
            println!("> checking docker and docker-compose...");
            check_docker();
            println!("> starting tower containers...");
            let mut source_dir = PathBuf::from(sub_matches.value_of("source_dir").expect("Currently we only support install from source code, please provide the --from-source flag."));
            if !source_dir.is_absolute() {
                source_dir = env::current_dir()
                    .expect("failed to get current directory")
                    .join(source_dir)
                    .canonicalize()
                    .expect("failed to canoicalize source directory");
            }
            start_from_source(&source_dir, sub_matches.is_present("force"));
        }
        _ => {}
    };
}

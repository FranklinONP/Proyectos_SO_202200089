use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use serde_json;
use chrono::Utc;
use reqwest::Client;
use tokio::signal;

#[derive(Debug, Serialize, Deserialize)]
struct SystemInfo {
    #[serde(rename = "SystemInfo")]
    system_info: SystemInfoDetails,
    #[serde(rename = "Containers")]
    containers: Vec<Container>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SystemInfoDetails {
    #[serde(rename = "TotalRAM_MB")]
    total_ram_mb: u64,
    #[serde(rename = "FreeRAM_MB")]
    free_ram_mb: u64,
    #[serde(rename = "UsedRAM_MB")]
    used_ram_mb: u64,
    #[serde(rename = "TotalCPUUsagePercent")]
    total_cpu_usage_percent: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
struct Container {
    #[serde(rename = "ID")]
    id: String,
    #[serde(rename = "PID")]
    pid: u32,
    #[serde(rename = "Cmdline")]
    cmdline: String,
    #[serde(rename = "MemoryUsageMB")]
    memory_usage_mb: u64,
    #[serde(rename = "CPUUsagePercent")]
    cpu_usage_percent: f64,
    #[serde(rename = "ReadBytesMB")]
    read_bytes_mb: u64,
    #[serde(rename = "WriteBytesMB")]
    write_bytes_mb: u64,
    #[serde(rename = "TotalIOBytesMB")]
    total_io_bytes_mb: u64,
    #[serde(skip)]
    creation_time: String,
    #[serde(default)]
    saved_at: String,
}

#[derive(Debug, Clone)]
struct DockerContainer {
    id: String,
    created: String,
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PersistentData {
    #[serde(rename = "stress --hdd 1")]
    stress_hdd: Vec<Container>,
    #[serde(rename = "stress --io 1")]
    stress_io: Vec<Container>,
    #[serde(rename = "stress --vm 1 --vm-…")]
    stress_vm: Vec<Container>,
    #[serde(rename = "stress --cpu 1")]
    stress_cpu: Vec<Container>,
}

#[derive(Serialize)]
struct DashboardPanel {
    title: String,
    #[serde(rename = "type")]
    type_field: String,
    datasource: String,
    grid_pos: GridPos,
    targets: Vec<Target>,
}

#[derive(Serialize)]
struct GridPos {
    h: i32,
    w: i32,
    x: i32,
    y: i32,
}

#[derive(Serialize)]
struct Target {
    ref_id: String,
    #[serde(rename = "type")]
    type_field: String,
    query_type: String,
    query: String,
    format: String,
}

#[derive(Serialize)]
struct Dashboard {
    title: String,
    panels: Vec<DashboardPanel>,
    editable: bool,
    schema_version: i32,
    version: i32,
}

#[derive(Serialize)]
struct DashboardWrapper {
    dashboard: Dashboard,
    overwrite: bool,
}

fn read_proc_file(file_name: &str) -> io::Result<String> {
    let path = Path::new("/proc").join(file_name);
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

fn parse_proc_to_struct(json_str: &str) -> Result<SystemInfo, serde_json::Error> {
    serde_json::from_str(json_str)
}

fn get_docker_containers() -> Vec<DockerContainer> {
    let output = Command::new("docker")
        .arg("ps")
        .arg("-a")
        .arg("--format")
        .arg("{{.ID}}\t{{.CreatedAt}}\t{{.Names}}")
        .output()
        .expect("Fallo al ejecutar docker ps");

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 {
                Some(DockerContainer {
                    id: parts[0].to_string(),
                    created: parts[1].to_string(),
                    name: parts[2].to_string(),
                })
            } else {
                None
            }
        })
        .collect()
}

fn kill_container(id: &str, name: &str) -> io::Result<()> {
    if name.contains("grafana") {
        println!("Contenedor Grafana {} no será eliminado", id);
        return Ok(());
    }
    
    let output = Command::new("sudo")
        .arg("docker")
        .arg("rm")
        .arg("-f")
        .arg(id)
        .output()?;

    if output.status.success() {
        println!("Contenedor borrado: ID={}, Nombre={}", id, name);
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Error al eliminar contenedor {}: {}", id, error),
        ))
    }
}

fn load_persistent_json(file_path: &str) -> PersistentData {
    if let Ok(mut file) = File::open(file_path) {
        let mut content = String::new();
        if file.read_to_string(&mut content).is_ok() {
            if let Ok(data) = serde_json::from_str(&content) {
                return data;
            }
        }
    }
    PersistentData {
        stress_hdd: Vec::new(),
        stress_io: Vec::new(),
        stress_vm: Vec::new(),
        stress_cpu: Vec::new(),
    }
}

fn save_persistent_json(file_path: &str, data: &PersistentData) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_path)?;
    let json = serde_json::to_string_pretty(data)?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

async fn send_to_grafana(client: &Client, _data: &PersistentData) -> Result<(), Box<dyn std::error::Error>> {
    let dashboard = DashboardWrapper {
        dashboard: Dashboard {
            title: "Container Metrics".to_string(),
            panels: vec![
                DashboardPanel {
                    title: "Disk Usage (HDD)".to_string(),
                    type_field: "timeseries".to_string(),
                    datasource: "InfinityDS".to_string(),
                    grid_pos: GridPos { h: 8, w: 12, x: 0, y: 0 },
                    targets: vec![Target {
                        ref_id: "A".to_string(),
                        type_field: "timeseries".to_string(),
                        query_type: "json".to_string(),
                        query: "$.['stress --hdd 1'][*].{value: TotalIOBytesMB, time: saved_at}".to_string(),
                        format: "time_series".to_string(),
                    }],
                },
                DashboardPanel {
                    title: "IO Usage".to_string(),
                    type_field: "timeseries".to_string(),
                    datasource: "InfinityDS".to_string(),
                    grid_pos: GridPos { h: 8, w: 12, x: 12, y: 0 },
                    targets: vec![
                        Target {
                            ref_id: "A".to_string(),
                            type_field: "timeseries".to_string(),
                            query_type: "json".to_string(),
                            query: "$.['stress --io 1'][*].{value: ReadBytesMB, time: saved_at}".to_string(),
                            format: "time_series".to_string(),
                        },
                        Target {
                            ref_id: "B".to_string(),
                            type_field: "timeseries".to_string(),
                            query_type: "json".to_string(),
                            query: "$.['stress --io 1'][*].{value: WriteBytesMB, time: saved_at}".to_string(),
                            format: "time_series".to_string(),
                        },
                    ],
                },
                DashboardPanel {
                    title: "RAM Usage".to_string(),
                    type_field: "timeseries".to_string(),
                    datasource: "InfinityDS".to_string(),
                    grid_pos: GridPos { h: 8, w: 12, x: 0, y: 8 },
                    targets: vec![Target {
                        ref_id: "A".to_string(),
                        type_field: "timeseries".to_string(),
                        query_type: "json".to_string(),
                        query: "$.['stress --vm 1 --vm-…'][*].{value: MemoryUsageMB, time: saved_at}".to_string(),
                        format: "time_series".to_string(),
                    }],
                },
                DashboardPanel {
                    title: "CPU Usage".to_string(),
                    type_field: "timeseries".to_string(),
                    datasource: "InfinityDS".to_string(),
                    grid_pos: GridPos { h: 8, w: 12, x: 12, y: 8 },
                    targets: vec![Target {
                        ref_id: "A".to_string(),
                        type_field: "timeseries".to_string(),
                        query_type: "json".to_string(),
                        query: "$.['stress --cpu 1'][*].{value: CPUUsagePercent, time: saved_at}".to_string(),
                        format: "time_series".to_string(),
                    }],
                },
            ],
            editable: true,
            schema_version: 36,
            version: 0,
        },
        overwrite: true,
    };

    let response = client
        .post("http://localhost:3000/api/dashboards/db")
        .header("Authorization", "Bearer glsa_6r784gBmXaSi7AWVhg0dATev5eJk4MlW_2c7eac84")
        .header("Content-Type", "application/json")
        .json(&dashboard)
        .send()
        .await?;

    if response.status().is_success() {
        println!("Dashboard actualizado en Grafana exitosamente");
    } else {
        eprintln!("Error al actualizar dashboard: {:?}", response.text().await?);
    }

    Ok(())
}

async fn manage_containers(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    let docker_containers = get_docker_containers();
    
    let system_info = read_proc_file("sysinfo")?;
    let system_info = parse_proc_to_struct(&system_info)?;

    let mut containers_with_metrics: Vec<(DockerContainer, Container)> = Vec::new();
    for dc in &docker_containers {
        if let Some(c) = system_info.containers.iter().find(|c| c.id == dc.id) {
            let mut container = c.clone();
            container.creation_time = dc.created.clone();
            containers_with_metrics.push((dc.clone(), container));
        }
    }

    let mut cpu_containers = Vec::new();
    let mut ram_containers = Vec::new();
    let mut io_containers = Vec::new();
    let mut disk_containers = Vec::new();
    
    for pair in containers_with_metrics {
        let cmdline = pair.1.cmdline.trim();
        match cmdline {
            "stress --cpu 1" => cpu_containers.push(pair),
            cmd if cmd.starts_with("stress --vm 1") => ram_containers.push(pair),
            "stress --io 1" => io_containers.push(pair),
            "stress --hdd 1" => disk_containers.push(pair),
            _ => {}
        }
    }

    cpu_containers.sort_by(|a, b| b.0.created.cmp(&a.0.created));
    ram_containers.sort_by(|a, b| b.0.created.cmp(&a.0.created));
    io_containers.sort_by(|a, b| b.0.created.cmp(&a.0.created));
    disk_containers.sort_by(|a, b| b.0.created.cmp(&a.0.created));

    let mut keep_containers = Vec::new();
    if let Some(cpu) = cpu_containers.first() { keep_containers.push(cpu.clone()); }
    if let Some(ram) = ram_containers.first() { keep_containers.push(ram.clone()); }
    if let Some(io) = io_containers.first() { keep_containers.push(io.clone()); }
    if let Some(disk) = disk_containers.first() { keep_containers.push(disk.clone()); }

    let keep_ids: Vec<String> = keep_containers.iter()
        .map(|(dc, _)| dc.id.clone())
        .collect();

    for dc in &docker_containers {
        if !keep_ids.contains(&dc.id) && !dc.name.contains("grafana") && dc.id != "N/A" {
            kill_container(&dc.id, &dc.name)?;
        }
    }

    let persistent_file = "persistent_containers.json";
    let mut persistent_data = load_persistent_json(persistent_file);
    let now = Utc::now().to_rfc3339();

    for (dc, container) in &mut keep_containers {
        container.saved_at = now.clone();
        match container.cmdline.trim() {
            "stress --hdd 1" => {
                println!("Contenedor guardado: ID={}, Nombre={}, Tipo=stress --hdd 1", dc.id, dc.name);
                persistent_data.stress_hdd.push(container.clone());
            }
            "stress --io 1" => {
                println!("Contenedor guardado: ID={}, Nombre={}, Tipo=stress --io 1", dc.id, dc.name);
                persistent_data.stress_io.push(container.clone());
            }
            cmd if cmd.starts_with("stress --vm 1") => {
                println!("Contenedor guardado: ID={}, Nombre={}, Tipo=stress --vm 1", dc.id, dc.name);
                persistent_data.stress_vm.push(container.clone());
            }
            "stress --cpu 1" => {
                println!("Contenedor guardado: ID={}, Nombre={}, Tipo=stress --cpu 1", dc.id, dc.name);
                persistent_data.stress_cpu.push(container.clone());
            }
            _ => {}
        }
    }

    save_persistent_json(persistent_file, &persistent_data)?;
    send_to_grafana(client, &persistent_data).await?;
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    println!("Presiona Ctrl+C para finalizar...");

    loop {
        if let Err(e) = manage_containers(&client).await {
            eprintln!("Error en manage_containers: {}", e);
        }
        
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(30)) => {},
            _ = signal::ctrl_c() => {
                println!("Programa finalizado");
                break;
            }
        }
    }

    Ok(())
}
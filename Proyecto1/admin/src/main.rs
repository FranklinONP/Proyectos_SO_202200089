use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use serde_json;
use chrono::{DateTime, Utc};
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
    type_: String,
    datasource: String,
    gridPos: GridPos,
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
    refId: String,
    #[serde(rename = "type")]
    type_: String,
    queryType: String,
    query: String,
    format: String,
}

#[derive(Serialize)]
struct Dashboard {
    title: String,
    panels: Vec<DashboardPanel>,
    editable: bool,
    schemaVersion: i32,
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
    println!("Contenido de /proc/sysinfo: {}", content);
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

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines()
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
    if name == "grafana" {
        println!("Contenedor {} es Grafana, no se eliminará.", id);
        return Ok(());
    }

    let output = Command::new("sudo")
        .arg("docker")
        .arg("rm")
        .arg("-f")
        .arg(id)
        .output()?;

    if output.status.success() {
        println!("Contenedor {} eliminado con éxito", id);
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(io::Error::new(io::ErrorKind::Other, format!("Error al eliminar contenedor {}: {}", id, error)))
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
        .truncate(true) // Esto sobrescribe, pero lo manejaremos en manage_containers
        .open(file_path)?;
    let json = serde_json::to_string_pretty(data)?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

async fn create_dashboard(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    let dashboard = DashboardWrapper {
        dashboard: Dashboard {
            title: "Container Metrics".to_string(),
            panels: vec![
                DashboardPanel {
                    title: "HDD Usage (Disk)".to_string(),
                    type_: "timeseries".to_string(),
                    datasource: "InfinityDS".to_string(),
                    gridPos: GridPos { h: 8, w: 12, x: 0, y: 0 },
                    targets: vec![Target {
                        refId: "A".to_string(),
                        type_: "timeseries".to_string(),
                        queryType: "json".to_string(),
                        query: "$.['stress --hdd 1'].[*].{value: TotalIOBytesMB, time: saved_at}".to_string(),
                        format: "time_series".to_string(),
                    }],
                },
                DashboardPanel {
                    title: "IO Usage".to_string(),
                    type_: "timeseries".to_string(),
                    datasource: "InfinityDS".to_string(),
                    gridPos: GridPos { h: 8, w: 12, x: 12, y: 0 },
                    targets: vec![
                        Target {
                            refId: "A".to_string(),
                            type_: "timeseries".to_string(),
                            queryType: "json".to_string(),
                            query: "$.['stress --io 1'].[*].{value: ReadBytesMB, time: saved_at}".to_string(),
                            format: "time_series".to_string(),
                        },
                        Target {
                            refId: "B".to_string(),
                            type_: "timeseries".to_string(),
                            queryType: "json".to_string(),
                            query: "$.['stress --io 1'].[*].{value: WriteBytesMB, time: saved_at}".to_string(),
                            format: "time_series".to_string(),
                        },
                    ],
                },
                DashboardPanel {
                    title: "VM Usage (RAM)".to_string(),
                    type_: "timeseries".to_string(),
                    datasource: "InfinityDS".to_string(),
                    gridPos: GridPos { h: 8, w: 12, x: 0, y: 8 },
                    targets: vec![Target {
                        refId: "A".to_string(),
                        type_: "timeseries".to_string(),
                        queryType: "json".to_string(),
                        query: "$.['stress --vm 1 --vm-…'].[*].{value: MemoryUsageMB, time: saved_at}".to_string(),
                        format: "time_series".to_string(),
                    }],
                },
                DashboardPanel {
                    title: "CPU Usage".to_string(),
                    type_: "timeseries".to_string(),
                    datasource: "InfinityDS".to_string(),
                    gridPos: GridPos { h: 8, w: 12, x: 12, y: 8 },
                    targets: vec![Target {
                        refId: "A".to_string(),
                        type_: "timeseries".to_string(),
                        queryType: "json".to_string(),
                        query: "$.['stress --cpu 1'].[*].{value: CPUUsagePercent, time: saved_at}".to_string(),
                        format: "time_series".to_string(),
                    }],
                },
            ],
            editable: true,
            schemaVersion: 36,
            version: 0,
        },
        overwrite: true,
    };

    let response = client
        .post("http://localhost:3000/api/dashboards/db")
        .header("Authorization", "Bearer glsa_JjXfkiIBErXNTmLDNT3UzkTbCaRbo0fY_4ece443e") // Reemplaza con tu token API
        .header("Content-Type", "application/json")
        .json(&dashboard)
        .send()
        .await?;

    if response.status().is_success() {
        println!("Dashboard creado exitosamente en Grafana.");
    } else {
        eprintln!("Error al crear el dashboard: {:?}", response.text().await?);
    }

    Ok(())
}

async fn manage_containers() {
    let docker_containers = get_docker_containers();
    println!("Contenedores encontrados: {:?}", docker_containers);

    let mut system_info = match read_proc_file("sysinfo") {
        Ok(json_str) => match parse_proc_to_struct(&json_str) {
            Ok(info) => info,
            Err(e) => {
                eprintln!("Error al parsear JSON: {}", e);
                return;
            }
        },
        Err(e) => {
            eprintln!("Error al leer /proc/sysinfo: {}", e);
            return;
        }
    };

    let mut non_grafana_containers: Vec<DockerContainer> = docker_containers
        .iter()
        .filter(|dc| dc.name != "grafana")
        .cloned()
        .collect();

    let mut containers_with_metrics: Vec<(DockerContainer, Container)> = Vec::new();
    for dc in &non_grafana_containers {
        if let Some(c) = system_info.containers.iter_mut().find(|c| c.id == dc.id) {
            c.creation_time = dc.created.clone();
            containers_with_metrics.push((dc.clone(), c.clone()));
        }
    }

    containers_with_metrics.sort_by(|a, b| b.0.created.cmp(&a.0.created));
    let mut latest_containers: Vec<(DockerContainer, Container)> = containers_with_metrics
        .into_iter()
        .take(4)
        .collect();

    let keep_ids: Vec<String> = latest_containers.iter().map(|(dc, _)| dc.id.clone()).collect();
    println!("Contenedores a conservar (más jóvenes, excluyendo Grafana): {:?}", keep_ids);

    let grafana_id = docker_containers
        .iter()
        .find(|dc| dc.name == "grafana")
        .map(|dc| dc.id.clone());

    for dc in &docker_containers {
        let is_grafana = grafana_id.as_ref().map_or(false, |id| id == &dc.id);
        if !keep_ids.contains(&dc.id) && !is_grafana && dc.id != "N/A" {
            if let Err(e) = kill_container(&dc.id, &dc.name) {
                eprintln!("Error al eliminar contenedor {}: {}", dc.id, e);
            }
        }
    }

    let persistent_file = "persistent_containers.json";
    let mut persistent_data = load_persistent_json(persistent_file);
    let now: DateTime<Utc> = Utc::now();
    let saved_at = now.to_rfc3339();

    for &mut (_, ref mut container) in &mut latest_containers {
        container.saved_at = saved_at.clone();
        let cmdline = container.cmdline.trim();
        if cmdline == "stress --hdd 1" {
            persistent_data.stress_hdd.push(container.clone());
        } else if cmdline == "stress --io 1" {
            persistent_data.stress_io.push(container.clone());
        } else if cmdline.starts_with("stress --vm 1") {
            persistent_data.stress_vm.push(container.clone());
        } else if cmdline == "stress --cpu 1" {
            persistent_data.stress_cpu.push(container.clone());
        } else {
            println!("Contenedor con Cmdline '{}' no coincide con ninguna categoría", cmdline);
        }
    }

    if let Err(e) = save_persistent_json(persistent_file, &persistent_data) {
        eprintln!("Error al guardar JSON persistente: {}", e);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    println!("Presiona Ctrl+C para finalizar...");
    let mut running = true;

    while running {
        manage_containers().await;
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(30)) => {},
            _ = signal::ctrl_c() => {
                println!("Recibido Ctrl+C, finalizando...");
                create_dashboard(&client).await?;
                running = false;
            }
        }
    }

    println!("Programa finalizado. Dashboard creado en Grafana.");
    Ok(())
}


//ya me crea algop mejor
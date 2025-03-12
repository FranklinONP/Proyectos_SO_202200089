use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;

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
    #[serde(skip)] // No serializamos este campo directamente en el JSON
    creation_time: String, // Añadido para guardar la fecha de creación
}

#[derive(Debug, Clone)]
struct DockerContainer {
    id: String,
    created: String,
}

// Estructura para el JSON persistente clasificado por tipo
#[derive(Debug, Serialize, Deserialize)]
struct PersistentData {
    #[serde(flatten)]
    containers_by_type: HashMap<String, Vec<Container>>,
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
        .arg("{{.ID}}\t{{.CreatedAt}}")
        .output()
        .expect("Fallo al ejecutar docker ps");

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 2 {
                Some(DockerContainer {
                    id: parts[0].to_string(),
                    created: parts[1].to_string(),
                })
            } else {
                None
            }
        })
        .collect()
}

fn kill_container(id: &str) -> io::Result<()> {
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
        containers_by_type: HashMap::new(),
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

// Función para clasificar contenedores según Cmdline (ajusta según tus 4 tipos)
fn classify_container(cmdline: &str) -> String {
    // Ejemplo básico: clasifica según palabras clave en Cmdline
    // Debes definir tus 4 tipos aquí según tu criterio
    if cmdline.contains("nginx") {
        "web".to_string()
    } else if cmdline.contains("python") {
        "app".to_string()
    } else if cmdline.contains("java") {
        "java".to_string()
    } else {
        "other".to_string()
    }
}

fn manage_containers() {
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

    let mut containers_with_metrics: Vec<(DockerContainer, Container)> = Vec::new();
    for dc in &docker_containers {
        if let Some(c) = system_info.containers.iter_mut().find(|c| c.id == dc.id) {
            c.creation_time = dc.created.clone(); // Añadimos creation_time al contenedor
            containers_with_metrics.push((dc.clone(), c.clone()));
        }
    }

    containers_with_metrics.sort_by(|a, b| b.0.created.cmp(&a.0.created));
    let latest_containers: Vec<(DockerContainer, Container)> = containers_with_metrics
        .into_iter()
        .take(4)
        .collect();

    let keep_ids: Vec<String> = latest_containers.iter().map(|(dc, _)| dc.id.clone()).collect();
    println!("Contenedores a conservar (más jóvenes): {:?}", keep_ids);

    for dc in &docker_containers {
        if !keep_ids.contains(&dc.id) && dc.id != "N/A" {
            if let Err(e) = kill_container(&dc.id) {
                eprintln!("Error al eliminar contenedor {}: {}", dc.id, e);
            }
        }
    }

    let persistent_file = "persistent_containers.json";
    let mut persistent_data = load_persistent_json(persistent_file);

    for (_, container) in &latest_containers {
        let container_type = classify_container(&container.cmdline);
        let containers_list = persistent_data.containers_by_type
            .entry(container_type)
            .or_insert_with(Vec::new);
        
        if !containers_list.iter().any(|c| c.id == container.id) {
            containers_list.push(container.clone());
        }
    }

    if let Err(e) = save_persistent_json(persistent_file, &persistent_data) {
        eprintln!("Error al guardar JSON persistente: {}", e);
    }
}

fn main() {
    loop {
        manage_containers();
        thread::sleep(Duration::from_secs(30));
    }
}
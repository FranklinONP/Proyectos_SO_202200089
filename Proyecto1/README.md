# Proyectos_SO_202200089

Comandos

Compilar modulo kernel
make

Borrar compilado de modulo kernel
make clean

subir modulo de kernel
sudo insmod sysinfo.ko

ver que modulo esta subido
lsmod | grep sysinfo

elimiar modulo/bajarlo

sudo rmmod sysinfo

ver contenido de json
cat /proc/sysinfo

Borrar todos los contenedores 
docker rm -f $(docker ps -aq)

docker-compose up


saber consumo real...
    ==> ps -eo pid,ppid,rss,comm | grep containerd-shim





rust

cargo build

cargo run


docker run -d -p 3000:3000 --name grafana \
  -v /home/franklin-noj/Escritorio/Proyectos_SO_202200089/Proyecto1/admin/src/persistent_containers.json:/var/lib/grafana/persistent_containers.json:ro \
  grafana/grafana:latest


docker run -d -p 3000:3000 --name grafana \
  grafana/grafana:latest


docker-compose up -d
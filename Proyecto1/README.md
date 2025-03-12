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
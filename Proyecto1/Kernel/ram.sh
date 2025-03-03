#!/bin/bash

# Función para generar un nombre aleatorio para el contenedor
generate_random_name() {
    cat /dev/urandom | tr -dc 'a-zA-Z0-9' | fold -w 10 | head -n 1
}

# Crear contenedor de tipo --vm 1 --vm-bytes 128M
container_name=$(generate_random_name)
docker run -d --name $container_name alpine-stress stress --vm 1 --vm-bytes 128M
echo "Contenedor $container_name creado con configuración: --vm 1 --vm-bytes 128M"

# Registrar ejecución en un log
echo "Script ejecutado el $(date)" >> /home/franklin-noj/Documentos/Universidad/7mo_Semestre/Sopes1/Lab/Proyectos_SO_202200089/Proyecto1/log.txt

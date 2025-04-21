#!/bin/bash

# Función para generar un nombre aleatorio para el contenedor
generate_random_name() {
    cat /dev/urandom | tr -dc 'a-zA-Z0-9' | fold -w 10 | head -n 1
}

# Tipos de contenedores que se pueden crear
declare -a container_types=("--cpu 1" "--vm 1 --vm-bytes 128M" "--io 1" "--hdd 1")

# Crear 10 contenedores de manera aleatoria
for i in {1..10}
do
    # Seleccionar un tipo de contenedor aleatorio
    container_type=${container_types[$RANDOM % ${#container_types[@]}]}
    
    # Generar un nombre aleatorio para el contenedor
    container_name=$(generate_random_name)
    
    # Crear el contenedor con el tipo seleccionado
    docker run -d --name $container_name containerstack/alpine-stress stress $container_type
    
    echo "Contenedor $container_name creado con configuración: $container_type"
done

# Registrar ejecución en un log
echo "Script ejecutado el $(date)" >> ~/Escritorio/Proyectos_SO_202200089/Proyecto1/Kernel/log.txt
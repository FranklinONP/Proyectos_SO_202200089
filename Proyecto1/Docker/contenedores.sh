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
    docker run -d --name $container_name alpine-stress stress $container_type
    
    # Programar la eliminación del contenedor después de 30 segundos
    (sleep 60; docker rm -f $container_name) &
    
    echo "Contenedor $container_name creado y programado para eliminarse en 30 segundos."
done

#!/bin/bash
echo "Script ejecutado el $(date)" >> /home/franklin-noj/Documentos/Universidad/7mo_Semestre/Sopes1/Lab/Proyectos_SO_202200089/Proyecto1/log.txt

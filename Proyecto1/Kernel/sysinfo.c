#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/string.h>
#include <linux/init.h>
#include <linux/proc_fs.h>
#include <linux/seq_file.h>
#include <linux/mm.h>
#include <linux/sched.h>
#include <linux/timer.h>
#include <linux/jiffies.h>
#include <linux/uaccess.h>
#include <linux/tty.h>
#include <linux/sched/signal.h>
#include <linux/fs.h>
#include <linux/slab.h>
#include <linux/sched/mm.h>
#include <linux/binfmts.h>
#include <linux/timekeeping.h>
#include <linux/cpumask.h>
#include <linux/cpu.h>
#include <linux/kernel_stat.h>

MODULE_LICENSE("GPL");
MODULE_AUTHOR("Franklin");
MODULE_DESCRIPTION("Modulo para leer información de memoria y CPU en JSON");
MODULE_VERSION("1.0");

#define PROC_NAME "sysinfo"
#define MAX_CMDLINE_LENGTH 256
#define CONTAINER_ID_LENGTH 64

// Función para obtener la línea de comandos de un proceso
static char *get_process_cmdline(struct task_struct *task) {
    struct mm_struct *mm;
    char *cmdline, *p;
    unsigned long arg_start, arg_end, env_start;
    int i, len;

    // Reservamos memoria para la línea de comandos
    cmdline = kmalloc(MAX_CMDLINE_LENGTH, GFP_KERNEL);
    if (!cmdline)
        return NULL;

    // Obtenemos la información de memoria
    mm = get_task_mm(task);
    if (!mm) {
        kfree(cmdline);
        return NULL;
    }

    // Obtenemos las direcciones de inicio y fin de los argumentos y el entorno
    down_read(&mm->mmap_lock);
    arg_start = mm->arg_start;
    arg_end = mm->arg_end;
    env_start = mm->env_start;
    up_read(&mm->mmap_lock);

    // Obtenemos la longitud de la línea de comandos
    len = arg_end - arg_start;
    if (len > MAX_CMDLINE_LENGTH - 1)
        len = MAX_CMDLINE_LENGTH - 1;

    // Leemos la línea de comandos de la memoria virtual del proceso
    if (access_process_vm(task, arg_start, cmdline, len, 0) != len) {
        mmput(mm);
        kfree(cmdline);
        return NULL;
    }

    // Agregamos un caracter nulo al final de la línea de comandos
    cmdline[len] = '\0';

    // Reemplazamos caracteres nulos por espacios
    p = cmdline;
    for (i = 0; i < len; i++)
        if (p[i] == '\0')
            p[i] = ' ';

    // Liberamos la estructura mm_struct
    mmput(mm);
    return cmdline;
}

// Función para calcular el uso de CPU del sistema
static unsigned long get_cpu_usage(void) {
    unsigned long user, nice, system, idle, iowait, irq, softirq, steal;
    unsigned long total_time, idle_time, cpu_usage;
    struct kernel_cpustat cpu_stat;

    // Obtenemos las estadísticas de CPU
    cpu_stat = kcpustat_cpu(0);
    user = cpu_stat.cpustat[CPUTIME_USER];
    nice = cpu_stat.cpustat[CPUTIME_NICE];
    system = cpu_stat.cpustat[CPUTIME_SYSTEM];
    idle = cpu_stat.cpustat[CPUTIME_IDLE];
    iowait = cpu_stat.cpustat[CPUTIME_IOWAIT];
    irq = cpu_stat.cpustat[CPUTIME_IRQ];
    softirq = cpu_stat.cpustat[CPUTIME_SOFTIRQ];
    steal = cpu_stat.cpustat[CPUTIME_STEAL];

    // Calculamos el tiempo total de CPU
    total_time = user + nice + system + idle + iowait + irq + softirq + steal;
    idle_time = idle + iowait;

    // Calculamos el uso de CPU
    cpu_usage = 100 - ((idle_time * 100) / total_time);

    return cpu_usage;
}

// Función para obtener la información de I/O de un proceso
static void get_io_info(struct task_struct *task, unsigned long *read_bytes, unsigned long *write_bytes) {
    struct task_io_accounting io = task->ioac;

    // Obtenemos los bytes leídos y escritos
    *read_bytes = io.read_bytes;
    *write_bytes = io.write_bytes;
}

// Función para obtener el uso de memoria de un proceso
static unsigned long get_process_memory_usage(struct task_struct *task) {
    unsigned long rss = 0;

    if (task->mm) {
        rss = get_mm_rss(task->mm) << (PAGE_SHIFT - 10); // Memoria residente en KB
    }

    return rss;
}

// Función recursiva para sumar el uso de memoria de todos los procesos ligados al contenedor
static unsigned long get_container_memory_usage(struct task_struct *task) {
    struct task_struct *child_task;
    struct list_head *child_list;
    unsigned long total_memory = 0;

    // Sumamos la memoria del proceso actual
    total_memory += get_process_memory_usage(task);

    // Iteramos sobre los hijos del proceso actual
    list_for_each(child_list, &task->children) {
        child_task = list_entry(child_list, struct task_struct, sibling);

        // Sumamos la memoria de los procesos hijos
        total_memory += get_container_memory_usage(child_task);
    }

    return total_memory;
}

// Función recursiva para encontrar el proceso del contenedor
static void find_container_process(struct task_struct *task, struct seq_file *m, struct sysinfo *si, unsigned long total_jiffies, int *first_process) {
    struct task_struct *child_task;
    struct list_head *child_list;

    // Iteramos sobre los hijos del proceso actual
    list_for_each(child_list, &task->children) {
        child_task = list_entry(child_list, struct task_struct, sibling);

        // Si el proceso hijo no es containerd-shim, lo consideramos como el proceso del contenedor
        if (strcmp(child_task->comm, "containerd-shim") != 0) {
            unsigned long memory_usage = 0;
            unsigned long cpu_usage = 0;
            unsigned long read_bytes = 0, write_bytes = 0;
            char *cmdline = NULL;

            // Obtenemos el uso de memoria de todos los procesos ligados al contenedor
            memory_usage = get_container_memory_usage(child_task);

            // Obtenemos el tiempo total de CPU de un proceso
            unsigned long total_time = child_task->utime + child_task->stime;
            cpu_usage = (total_time * 10000) / total_jiffies;

            // Obtenemos la línea de comandos
            cmdline = get_process_cmdline(child_task);

            // Obtenemos la información de I/O
            get_io_info(child_task, &read_bytes, &write_bytes);

            if (!*first_process) {
                seq_printf(m, ",\n");
            } else {
                *first_process = 0;
            }

            seq_printf(m, " {\n");
            seq_printf(m, " \"PID\": %d,\n", child_task->pid);
            seq_printf(m, " \"Name\": \"%s\",\n", child_task->comm);
            seq_printf(m, " \"Cmdline\": \"%s\",\n", cmdline ? cmdline : "N/A");
            seq_printf(m, " \"MemoryUsage\": %lu,\n", memory_usage); // Memoria en KB
            seq_printf(m, " \"CPUUsage\": %lu.%02lu,\n", cpu_usage / 100, cpu_usage % 100);
            seq_printf(m, " \"IOReadBytes\": %lu,\n", read_bytes);
            seq_printf(m, " \"IOWriteBytes\": %lu\n", write_bytes);
            seq_printf(m, " }");

            // Liberamos la memoria de la línea de comandos
            if (cmdline) {
                kfree(cmdline);
            }
        }

        // Llamada recursiva para buscar en los hijos de este proceso
        find_container_process(child_task, m, si, total_jiffies, first_process);
    }
}
// Función para mostrar la información en el archivo /proc en formato JSON
static int sysinfo_show(struct seq_file *m, void *v) {
    struct sysinfo si;
    struct task_struct *task;
    unsigned long total_jiffies = jiffies;
    int first_process = 1;

    // Obtenemos la información de memoria
    si_meminfo(&si);

    // Convertimos la memoria de páginas a KB
    unsigned long total_mem_kb = (si.totalram * si.mem_unit) / 1024;
    unsigned long free_mem_kb = (si.freeram * si.mem_unit) / 1024;
    unsigned long used_mem_kb = total_mem_kb - free_mem_kb;

    // Obtenemos el uso de CPU
    unsigned long cpu_usage = get_cpu_usage();

    seq_printf(m, "{\n");
    seq_printf(m, "\"SystemInfo\": {\n");
    seq_printf(m, "\"TotalMemoryKB\": %lu,\n", total_mem_kb);
    seq_printf(m, "\"FreeMemoryKB\": %lu,\n", free_mem_kb);
    seq_printf(m, "\"UsedMemoryKB\": %lu,\n", used_mem_kb);
    seq_printf(m, "\"CPUUsage\": %lu\n", cpu_usage);
    seq_printf(m, "},\n");
    seq_printf(m, "\"Processes\": [\n");

    // Iteramos sobre los procesos
    for_each_process(task) {
        if (strcmp(task->comm, "containerd-shim") == 0) {
            // Buscamos el proceso del contenedor dentro de los hijos de containerd-shim
            find_container_process(task, m, &si, total_jiffies, &first_process);
        }
    }

    seq_printf(m, "\n]\n}\n");
    return 0;
}

// Función que se ejecuta al abrir el archivo /proc
static int sysinfo_open(struct inode *inode, struct file *file) {
    return single_open(file, sysinfo_show, NULL);
}

// Estructura que contiene las operaciones del archivo /proc
static const struct proc_ops sysinfo_ops = {
    .proc_open = sysinfo_open,
    .proc_read = seq_read,
};

// Función de inicialización del módulo
static int __init sysinfo_init(void) {
    proc_create(PROC_NAME, 0, NULL, &sysinfo_ops);
    printk(KERN_INFO "sysinfo.json modulo cargado\n");
    return 0;
}

// Función de limpieza del módulo
static void __exit sysinfo_exit(void) {
    remove_proc_entry(PROC_NAME, NULL);
    printk(KERN_INFO "sysinfo.json modulo desinstalado\n");
}

module_init(sysinfo_init);
module_exit(sysinfo_exit);
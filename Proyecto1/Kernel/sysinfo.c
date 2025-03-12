#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/string.h>
#include <linux/init.h>
#include <linux/proc_fs.h>
#include <linux/seq_file.h>
#include <linux/mm.h>
#include <linux/sched.h>
#include <linux/sched/task.h>
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

MODULE_LICENSE("GPL");
MODULE_AUTHOR("Tu Nombre");
MODULE_DESCRIPTION("Modulo para leer informacion de contenedores en JSON");
MODULE_VERSION("1.0");

#define PROC_NAME "sysinfo"
#define MAX_CMDLINE_LENGTH 256
#define CONTAINER_ID_LENGTH 13

static char *get_child_cmdline(struct task_struct *task) {
    struct task_struct *child;
    struct mm_struct *mm;
    char *cmdline, *p;
    unsigned long arg_start, arg_end;
    int i, len;

    cmdline = kmalloc(MAX_CMDLINE_LENGTH, GFP_KERNEL);
    if (!cmdline)
        return NULL;

    rcu_read_lock();
    list_for_each_entry_rcu(child, &task->children, sibling) {
        if (child) {
            break;
        }
    }
    rcu_read_unlock();

    if (!child) {
        kfree(cmdline);
        return NULL;
    }

    mm = get_task_mm(child);
    if (!mm) {
        kfree(cmdline);
        return NULL;
    }

    down_read(&mm->mmap_lock);
    arg_start = mm->arg_start;
    arg_end = mm->arg_end;
    up_read(&mm->mmap_lock);

    len = min(arg_end - arg_start, (unsigned long)(MAX_CMDLINE_LENGTH - 1));
    if (access_process_vm(child, arg_start, cmdline, len, 0) != len) {
        mmput(mm);
        kfree(cmdline);
        return NULL;
    }

    cmdline[len] = '\0';
    p = cmdline;
    for (i = 0; i < len; i++)
        if (p[i] == '\0')
            p[i] = ' ';

    mmput(mm);
    return cmdline;
}

static char *extract_container_id(const char *cmdline) {
    char *id_start, *id_end;
    char *id = kmalloc(CONTAINER_ID_LENGTH, GFP_KERNEL);
    int id_len;

    if (!cmdline || !id)
        return NULL;

    id_start = strstr(cmdline, "-id");
    if (!id_start)
        goto fail;

    id_start += 4;
    while (*id_start == ' ')
        id_start++;

    id_end = id_start;
    while (*id_end && *id_end != ' ')
        id_end++;

    id_len = min((int)(id_end - id_start), CONTAINER_ID_LENGTH - 1);
    strncpy(id, id_start, id_len);
    id[id_len] = '\0';

    return id;

fail:
    kfree(id);
    return NULL;
}

static char *get_process_cmdline(struct task_struct *task) {
    struct mm_struct *mm;
    char *cmdline, *p;
    unsigned long arg_start, arg_end;
    int i, len;

    cmdline = kmalloc(MAX_CMDLINE_LENGTH, GFP_KERNEL);
    if (!cmdline)
        return NULL;

    mm = get_task_mm(task);
    if (!mm) {
        kfree(cmdline);
        return NULL;
    }

    down_read(&mm->mmap_lock);
    arg_start = mm->arg_start;
    arg_end = mm->arg_end;
    up_read(&mm->mmap_lock);

    len = min(arg_end - arg_start, (unsigned long)(MAX_CMDLINE_LENGTH - 1));
    if (access_process_vm(task, arg_start, cmdline, len, 0) != len) {
        mmput(mm);
        kfree(cmdline);
        return NULL;
    }

    cmdline[len] = '\0';
    p = cmdline;
    for (i = 0; i < len; i++)
        if (p[i] == '\0')
            p[i] = ' ';

    mmput(mm);
    return cmdline;
}

static unsigned long get_process_memory_usage(struct task_struct *task) {
    unsigned long rss = 0;

    if (task->mm) {
        rss = get_mm_rss(task->mm);
        rss = (rss * PAGE_SIZE) >> 20; // Convertir a MB
    }

    return rss;
}

static unsigned long get_container_memory_usage(struct task_struct *task) {
    struct task_struct *child;
    unsigned long total_memory = 0;

    total_memory += get_process_memory_usage(task);

    rcu_read_lock();
    list_for_each_entry_rcu(child, &task->children, sibling) {
        get_task_struct(child);
        total_memory += get_container_memory_usage(child);
        put_task_struct(child);
    }
    rcu_read_unlock();

    return total_memory;
}

static unsigned long get_process_cpu_time(struct task_struct *task) {
    return task->utime + task->stime;
}

static unsigned long get_container_cpu_time(struct task_struct *task) {
    struct task_struct *child;
    unsigned long total_cpu_time = 0;

    total_cpu_time += get_process_cpu_time(task);

    rcu_read_lock();
    list_for_each_entry_rcu(child, &task->children, sibling) {
        get_task_struct(child);
        total_cpu_time += get_container_cpu_time(child);
        put_task_struct(child);
    }
    rcu_read_unlock();

    return total_cpu_time;
}

// Función para obtener la información de I/O de un proceso
static void get_io_info(struct task_struct *task, unsigned long *read_bytes, unsigned long *write_bytes) {
    struct task_io_accounting io = task->ioac;

    *read_bytes = io.read_bytes >> 20;  // Convertir a MB
    *write_bytes = io.write_bytes >> 20; // Convertir a MB
}

// Función para sumar el I/O de todos los procesos del contenedor
static void get_container_io_usage(struct task_struct *task, unsigned long *total_read_bytes, unsigned long *total_write_bytes) {
    struct task_struct *child;
    unsigned long read_bytes = 0, write_bytes = 0;

    get_io_info(task, &read_bytes, &write_bytes);
    *total_read_bytes += read_bytes;
    *total_write_bytes += write_bytes;

    rcu_read_lock();
    list_for_each_entry_rcu(child, &task->children, sibling) {
        get_task_struct(child);
        get_container_io_usage(child, total_read_bytes, total_write_bytes);
        put_task_struct(child);
    }
    rcu_read_unlock();
}

static int sysinfo_show(struct seq_file *m, void *v) {
    struct sysinfo si;
    struct task_struct *task;
    unsigned long total_jiffies = jiffies;
    int first_process = 1;
    unsigned long total_system_cpu_time = 0;

    si_meminfo(&si);

    for_each_process(task) {
        total_system_cpu_time += task->utime + task->stime;
    }
    unsigned long total_cpu_percent = (total_system_cpu_time * 10000) / total_jiffies;

    unsigned long total_ram_mb = (si.totalram * PAGE_SIZE) >> 20;
    unsigned long free_ram_mb = (si.freeram * PAGE_SIZE) >> 20;
    unsigned long used_ram_mb = total_ram_mb - free_ram_mb;

    seq_printf(m, "{\n");
    seq_printf(m, "  \"SystemInfo\": {\n");
    seq_printf(m, "    \"TotalRAM_MB\": %lu,\n", total_ram_mb);
    seq_printf(m, "    \"FreeRAM_MB\": %lu,\n", free_ram_mb);
    seq_printf(m, "    \"UsedRAM_MB\": %lu,\n", used_ram_mb);
    char cpu_usage_str[16];  // Buffer para almacenar el texto del porcentaje
        snprintf(cpu_usage_str, sizeof(cpu_usage_str), "%lu.%02lu", 
                total_cpu_percent / 100, total_cpu_percent % 100);

        seq_printf(m, "    \"TotalCPUUsagePercent\": \"%s\"\n", cpu_usage_str);

    seq_printf(m, "  },\n");
    seq_printf(m, "  \"Containers\": [\n");

    rcu_read_lock();
    for_each_process(task) {
        if (strcmp(task->comm, "containerd-shim") == 0) {
            unsigned long mem_usage_mb = 0;
            unsigned long container_cpu_time = 0;
            unsigned long total_read_bytes = 0;
            unsigned long total_write_bytes = 0;
            char *cmdline = NULL;
            char *container_id = NULL;
            char *child_cmdline = NULL;

            get_task_struct(task);
            mem_usage_mb = get_container_memory_usage(task);
            container_cpu_time = get_container_cpu_time(task);
            get_container_io_usage(task, &total_read_bytes, &total_write_bytes);
            put_task_struct(task);

            unsigned long cpu_usage = total_system_cpu_time ? (container_cpu_time * 10000) / total_system_cpu_time : 0;

            cmdline = get_process_cmdline(task);
            if (cmdline)
                container_id = extract_container_id(cmdline);
            child_cmdline = get_child_cmdline(task);

            if (!first_process) {
                seq_printf(m, ",\n");
            } else {
                first_process = 0;
            }

            seq_printf(m, "  {\n");
            seq_printf(m, "    \"ID\": \"%s\",\n", container_id ? container_id : "N/A");
            seq_printf(m, "    \"PID\": %d,\n", task->pid);
            seq_printf(m, "    \"Cmdline\": \"%s\",\n", child_cmdline ? child_cmdline : "N/A");
            seq_printf(m, "    \"MemoryUsageMB\": %lu,\n", mem_usage_mb);
            seq_printf(m, "    \"CPUUsagePercent\": %lu.%02lu,\n", cpu_usage / 100, cpu_usage % 100);
            seq_printf(m, "    \"ReadBytesMB\": %lu,\n", total_read_bytes);
            seq_printf(m, "    \"WriteBytesMB\": %lu,\n", total_write_bytes);
            seq_printf(m, "    \"TotalIOBytesMB\": %lu\n", total_read_bytes + total_write_bytes);
            seq_printf(m, "  }");

            if (cmdline)
                kfree(cmdline);
            if (container_id)
                kfree(container_id);
            if (child_cmdline)
                kfree(child_cmdline);
        }
    }
    rcu_read_unlock();

    seq_printf(m, "\n  ]\n");
    seq_printf(m, "}\n");
    return 0;
}

static int sysinfo_open(struct inode *inode, struct file *file) {
    return single_open(file, sysinfo_show, NULL);
}

static const struct proc_ops sysinfo_ops = {
    .proc_open = sysinfo_open,
    .proc_read = seq_read,
    .proc_lseek = seq_lseek,
    .proc_release = single_release,
};

static int __init sysinfo_init(void) {
    proc_create(PROC_NAME, 0, NULL, &sysinfo_ops);
    printk(KERN_INFO "sysinfo modulo cargado\n");
    return 0;
}

static void __exit sysinfo_exit(void) {
    remove_proc_entry(PROC_NAME, NULL);
    printk(KERN_INFO "sysinfo modulo desinstalado\n");
}

module_init(sysinfo_init);
module_exit(sysinfo_exit);
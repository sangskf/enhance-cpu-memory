use clap::{Parser, Subcommand};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::Duration;
use sysinfo::{System, SystemExt, CpuExt};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process;
use std::str::FromStr;
#[cfg(unix)]
use fork;
use bytesize::ByteSize;

#[derive(Parser)]
#[command(author, version, about = "一个简易的CPU和内存负载工具", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// 要使用的CPU核心数量，默认为系统核心数的一半（至少为1）
    #[arg(short, long, default_value_t = std::cmp::max(1, num_cpus::get() / 2))]
    cores: usize,

    /// 要占用的内存大小（例如："1G"或"512M"）
    #[arg(short, long)]
    memory: Option<String>,

    /// 是否在后台运行
    #[arg(short, long)]
    background: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// 查看当前CPU和内存使用率
    Status,
    
    /// 启动CPU和内存负载
    Start {
        /// 要使用的CPU核心数量，默认为系统核心数的一半（至少为1）
        #[arg(short, long, default_value_t = std::cmp::max(1, num_cpus::get() / 2))]
        cores: usize,
        
        /// 要占用的内存大小（例如："1G"或"512M"）
        #[arg(short, long)]
        memory: Option<String>,
        
        /// 是否在后台运行
        #[arg(short, long)]
        background: bool,
    },
    
    /// 停止正在运行的负载
    Stop,
}

// 获取PID文件路径
fn get_pid_file() -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push("enhancecpu.pid");
    path
}

// 保存PID到文件
fn save_pid() -> std::io::Result<()> {
    let pid = process::id().to_string();
    let pid_file = get_pid_file();
    let mut file = File::create(pid_file)?;
    file.write_all(pid.as_bytes())?;
    Ok(())
}

// 读取PID文件
fn read_pid() -> Option<u32> {
    let pid_file = get_pid_file();
    if !pid_file.exists() {
        return None;
    }
    
    let mut file = match File::open(pid_file) {
        Ok(file) => file,
        Err(_) => return None,
    };
    
    let mut pid_str = String::new();
    if file.read_to_string(&mut pid_str).is_err() {
        return None;
    }
    
    pid_str.trim().parse::<u32>().ok()
}

// 删除PID文件
fn remove_pid_file() -> std::io::Result<()> {
    let pid_file = get_pid_file();
    if pid_file.exists() {
        std::fs::remove_file(pid_file)?;
    }
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    
    match &cli.command {
        Some(Commands::Status) => {
            show_cpu_status();
        },
        Some(Commands::Start { cores, memory, background }) => {
            // 检查是否已经有实例在运行
            if let Some(pid) = read_pid() {
                println!("已有一个实例正在运行 (PID: {})。如需停止，请使用 'stop' 命令", pid);
                return;
            }
            
            // 保存当前进程的PID
            if let Err(e) = save_pid() {
                println!("警告：无法保存PID文件: {}", e);
            }
            
            // 启动负载
            start_load(*cores, memory.clone(), *background);
        },
        Some(Commands::Stop) => {
            // 读取PID并发送终止信号
            if let Some(pid) = read_pid() {
                #[cfg(unix)]
                {
                    use std::process::Command;
                    println!("正在停止CPU负载进程 (PID: {})...", pid);
                    let _ = Command::new("kill").arg(pid.to_string()).status();
                    let _ = remove_pid_file();
                }
                
                #[cfg(windows)]
                {
                    use std::process::Command;
                    println!("正在停止CPU负载进程 (PID: {})...", pid);
                    let _ = Command::new("taskkill").args(&["/PID", &pid.to_string(), "/F"]).status();
                    let _ = remove_pid_file();
                }
                
                println!("CPU负载已停止");
            } else {
                println!("没有找到正在运行的CPU负载进程");
            }
        },

        None => {
            // 检查是否已经有实例在运行
            if let Some(pid) = read_pid() {
                println!("已有一个实例正在运行 (PID: {})。如需停止，请使用 'stop' 命令", pid);
                return;
            }
            
            // 保存当前进程的PID
            if let Err(e) = save_pid() {
                println!("警告：无法保存PID文件: {}", e);
            }
            
            // 启动负载
            start_load(cli.cores, cli.memory, cli.background);
        }
    }
}

/// 启动系统负载
fn start_load(num_cores: usize, memory_size: Option<String>, background: bool) {
    // 设置中断处理
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
        println!("正在停止系统负载...");
        let _ = remove_pid_file();
    }).expect("无法设置Ctrl-C处理器");

    // 启动CPU负载
    let actual_cores = num_cores.min(num_cpus::get());
    println!("启动CPU负载，使用 {} 个核心", actual_cores);
    
    // 解析并分配内存
    let memory_vec = if let Some(size_str) = memory_size {
        match ByteSize::from_str(&size_str) {
            Ok(size) => {
                println!("分配内存: {}", size);
                Some(vec![0u8; size.as_u64() as usize])
            }
            Err(_) => {
                println!("警告：无效的内存大小格式，将不会分配内存");
                None
            }
        }
    } else {
        None
    };
    
    if background {
        #[cfg(unix)]
        {
            println!("程序将在后台运行，使用 'stop' 命令停止");
            match fork::daemon(false, false) {
                Ok(fork::Fork::Child) => {
                    // 子进程继续执行负载
                    // 重新保存PID，因为子进程PID不同
                    if let Err(e) = save_pid() {
                        // 在后台模式下，打印到控制台可能不可见，可以考虑日志记录
                        eprintln!("警告：无法在后台进程中保存PID文件: {}", e);
                    }
                    // 子进程继续执行下面的负载代码
                }
                Ok(fork::Fork::Parent(pid)) => {
                    // 父进程退出
                    println!("父进程退出，子进程 (PID: {}) 在后台运行", pid);
                    std::process::exit(0); // 确保父进程干净退出
                }
                Err(_) => {
                    println!("错误：无法 fork 进程以在后台运行");
                    let _ = remove_pid_file(); // 清理父进程创建的PID文件
                    return; // 无法后台运行，直接返回
                }
            }
        }
        #[cfg(not(unix))] // 或者 #[cfg(windows)] 如果只想针对Windows
        {
            // 在 Windows 上，后台运行通常意味着创建一个没有控制台窗口的新进程
            // 这超出了简单 fork 的范围。这里我们仅打印警告并继续在前台运行。
            println!("警告：后台运行模式 (-b) 在 Windows 上行为不同或不受支持，程序将继续在前台运行。");
            println!("如需在 Windows 后台运行，请考虑使用其他工具或方法（如 PowerShell Start-Process 或配置为 Windows 服务）。");
            // 不执行 fork，继续在前台运行
        }
    }
    
    let handles: Vec<_> = (0..actual_cores)
        .map(|i| {
            let running = running.clone();
            thread::spawn(move || {
                println!("启动工作线程 {}", i);
                cpu_intensive_task(running);
            })
        })
        .collect();
    
    // 定期显示系统状态
    let status_thread = {
        let running = running.clone();
        let memory_size = memory_vec.as_ref().map(|v| v.len());
        thread::spawn(move || {
            let mut sys = System::new_all();
            while running.load(Ordering::SeqCst) {
                sys.refresh_all();
                let avg_usage = sys.cpus().iter()
                    .take(actual_cores)
                    .map(|cpu| cpu.cpu_usage())
                    .sum::<f32>() / actual_cores as f32;
                
                println!("当前CPU使用率: {:.1}%", avg_usage);
                if let Some(size) = memory_size {
                    let total = sys.total_memory();
                    let used = sys.used_memory();
                    println!("当前内存使用: {:.1}GB / {:.1}GB (已分配: {:.1}GB)",
                        used as f64 / 1024.0 / 1024.0,
                        total as f64 / 1024.0 / 1024.0,
                        size as f64 / 1024.0 / 1024.0 / 1024.0);
                } else {
                    // 未指定内存大小时不显示内存信息
                }
                thread::sleep(Duration::from_secs(2));
            }
        })
    };
    
    // 等待所有线程完成
    for handle in handles {
        let _ = handle.join();
    }
    let _ = status_thread.join();
    
    // 内存会在这里自动释放
    drop(memory_vec);
    
    // 清理PID文件
    let _ = remove_pid_file();
}

/// 显示当前系统状态
fn show_cpu_status() {
    let mut sys = System::new_all();
    sys.refresh_all();
    
    println!("系统信息:");
    println!("CPU信息:");
    println!("总核心数: {}", sys.cpus().len());
    
    // 等待一秒以获取准确的系统使用率
    thread::sleep(Duration::from_secs(1));
    sys.refresh_all();
    
    for (i, cpu) in sys.cpus().iter().enumerate() {
        println!("核心 #{}: {:.1}%", i, cpu.cpu_usage());
    }
    
    let avg_usage = sys.cpus().iter()
        .map(|cpu| cpu.cpu_usage())
        .sum::<f32>() / sys.cpus().len() as f32;
    
    println!("平均CPU使用率: {:.1}%", avg_usage);
    
    // 显示内存信息
    let total = sys.total_memory();
    let used = sys.used_memory();
    let available = sys.available_memory();
    
    println!("\n内存信息:");
    println!("总内存: {:.1} GB", total as f64 / 1024.0 / 1024.0);
    println!("已用内存: {:.1} GB", used as f64 / 1024.0 / 1024.0);
    println!("可用内存: {:.1} GB", available as f64 / 1024.0 / 1024.0);
    println!("内存使用率: {:.1}%", (used as f64 / total as f64) * 100.0);
}

/// CPU密集型任务，用于提高CPU使用率
fn cpu_intensive_task(running: Arc<AtomicBool>) {
    // Explicitly specify the type of x as f32
    let mut x: f32 = 0.0001;
    while running.load(Ordering::SeqCst) {
        // 执行一些计算密集型操作
        x = x.sin().cos().sin().cos();
        // 防止编译器优化掉这个计算
        if x == 0.0 {
            println!("这不太可能发生");
        }
    }
}

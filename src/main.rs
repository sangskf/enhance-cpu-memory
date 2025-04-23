use clap::{Parser, Subcommand};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::Duration;
use sysinfo::{System, SystemExt, CpuExt};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process;
use fork;

#[derive(Parser)]
#[command(author, version, about = "一个简易的CPU负载工具", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// 要使用的CPU核心数量，默认为系统核心数的一半（至少为1）
    #[arg(short, long, default_value_t = std::cmp::max(1, num_cpus::get() / 2))]
    cores: usize,

    /// 是否在后台运行
    #[arg(short, long)]
    background: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// 查看当前CPU使用率
    Status,
    
    /// 启动CPU负载
    Start {
        /// 要使用的CPU核心数量，默认为系统核心数的一半（至少为1）
        #[arg(short, long, default_value_t = std::cmp::max(1, num_cpus::get() / 2))]
        cores: usize,
        
        /// 是否在后台运行
        #[arg(short, long)]
        background: bool,
    },
    
    /// 停止正在运行的CPU负载
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
        Some(Commands::Start { cores, background }) => {
            // 检查是否已经有实例在运行
            if let Some(pid) = read_pid() {
                println!("已有一个实例正在运行 (PID: {})。如需停止，请使用 'stop' 命令", pid);
                return;
            }
            
            // 保存当前进程的PID
            if let Err(e) = save_pid() {
                println!("警告：无法保存PID文件: {}", e);
            }
            
            // 启动CPU负载
            start_cpu_load(*cores, *background);
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
            
            // 启动CPU负载
            start_cpu_load(cli.cores, cli.background);
        }
    }
}

/// 启动CPU负载
fn start_cpu_load(num_cores: usize, background: bool) {
    // 设置中断处理
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
        println!("正在停止CPU负载...");
        let _ = remove_pid_file();
    }).expect("无法设置Ctrl-C处理器");

    // 启动CPU负载
    let actual_cores = num_cores.min(num_cpus::get());
    println!("启动CPU负载，使用 {} 个核心", actual_cores);
    
    if background {
        println!("程序将在后台运行，使用 'stop' 命令停止");
        // 在后台模式下，分离进程并退出主进程
        if let Ok(fork::Fork::Child) = fork::daemon(false, false) {
            // 子进程继续运行
        } else {
            // 父进程退出
            return;
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
    
    // 定期显示CPU使用率
    let status_thread = {
        let running = running.clone();
        thread::spawn(move || {
            let mut sys = System::new_all();
            while running.load(Ordering::SeqCst) {
                sys.refresh_cpu();
                let avg_usage = sys.cpus().iter()
                    .take(actual_cores)
                    .map(|cpu| cpu.cpu_usage())
                    .sum::<f32>() / actual_cores as f32;
                
                println!("当前CPU使用率: {:.1}%", avg_usage);
                thread::sleep(Duration::from_secs(2));
            }
        })
    };
    
    // 等待所有线程完成
    for handle in handles {
        let _ = handle.join();
    }
    let _ = status_thread.join();
    
    // 清理PID文件
    let _ = remove_pid_file();
}

/// 显示当前CPU状态
fn show_cpu_status() {
    let mut sys = System::new_all();
    sys.refresh_cpu();
    
    println!("CPU信息:");
    println!("总核心数: {}", sys.cpus().len());
    
    // 等待一秒以获取准确的CPU使用率
    thread::sleep(Duration::from_secs(1));
    sys.refresh_cpu();
    
    for (i, cpu) in sys.cpus().iter().enumerate() {
        println!("核心 #{}: {:.1}%", i, cpu.cpu_usage());
    }
    
    let avg_usage = sys.cpus().iter()
        .map(|cpu| cpu.cpu_usage())
        .sum::<f32>() / sys.cpus().len() as f32;
    
    println!("平均CPU使用率: {:.1}%", avg_usage);
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

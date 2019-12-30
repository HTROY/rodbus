use rodbus::error::details::ExceptionCode;
use rodbus::prelude::*;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;
use tokio::net::TcpListener;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::fmt::{Display, Formatter, Error};

struct Handler {
    coils: [bool; 100],
}
impl ServerHandler for Handler {
    fn read_coils(&mut self, range: AddressRange) -> Result<&[bool], ExceptionCode> {
        Self::get_range_of(self.coils.as_ref(), range)
    }

    fn read_discrete_inputs(&mut self, _: AddressRange) -> Result<&[bool], ExceptionCode> {
        Err(ExceptionCode::IllegalFunction)
    }

    fn read_holding_registers(&mut self, _: AddressRange) -> Result<&[u16], ExceptionCode> {
        Err(ExceptionCode::IllegalFunction)
    }

    fn read_input_registers(&mut self, _: AddressRange) -> Result<&[u16], ExceptionCode> {
        Err(ExceptionCode::IllegalFunction)
    }

    fn write_single_coil(&mut self, _: Indexed<CoilState>) -> Result<(), ExceptionCode> {
        Err(ExceptionCode::IllegalFunction)
    }

    fn write_single_register(&mut self, _: Indexed<RegisterValue>) -> Result<(), ExceptionCode> {
        Err(ExceptionCode::IllegalFunction)
    }
}

struct PerfOptions {
    pub num_sessions: u64,
    pub num_requests: u64,
}
impl PerfOptions {
    pub fn new(num_sessions: u64, num_requests: u64) -> Self {
        Self{num_sessions, num_requests}
    }

    pub fn total_num_requests(&self) -> u64 {
        self.num_sessions * self.num_requests
    }
}
impl Display for PerfOptions {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "{} sessions - {} requests", self.num_sessions, self.num_requests)
    }
}

async fn run_perf(options: &PerfOptions) {

    println!("START");
    let addr = SocketAddr::from_str("127.0.0.1:40000").unwrap();

    let handler = Handler {
        coils: [false; 100],
    }
    .wrap();
    let listener = TcpListener::bind(addr).await.unwrap();

    tokio::spawn(create_tcp_server_task(
        options.num_sessions as usize,
        listener,
        ServerHandlerMap::single(UnitId::new(1), handler),
    ));

    // now spawn a bunch of clients
    let mut sessions: Vec<Session> = Vec::new();
    for _ in 0..options.num_sessions {
        sessions.push(
            spawn_tcp_client_task(addr, 10, strategy::default())
                .create_session(UnitId::new(1), Duration::from_secs(1)),
        );
    }

    let mut query_tasks: Vec<tokio::task::JoinHandle<()>> = Vec::new();

    // spawn tasks that make a query 1000 times
    for mut session in sessions {
        let num_requests = options.num_requests;
        let handle: tokio::task::JoinHandle<()> = tokio::spawn(async move {
            for _ in 0..num_requests {
                session.read_coils(AddressRange::new(0, 100)).await.unwrap();
            }
        });
        query_tasks.push(handle);
    }

    for handle in query_tasks {
        handle.await.unwrap();
    }

    println!("STOP");
}

fn perf_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("perf_tests");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(30));
    for perf_options in [PerfOptions::new(10, 1000)].iter() {
        group.throughput(Throughput::Elements(perf_options.total_num_requests()));
        group.bench_with_input(BenchmarkId::new("perf", perf_options), perf_options, |b, perf_options| {
            b.iter(|| {
                println!("START2");
                {
                    let mut rt = tokio::runtime::Builder::new()
                        .threaded_scheduler()
                        .enable_all()
                        .build()
                        .unwrap();


                    rt.block_on(run_perf(perf_options));
                }
                println!("STOP2");
            });
        });
    }
    group.finish();
}

criterion_group!(benches, perf_bench);
criterion_main!(benches);

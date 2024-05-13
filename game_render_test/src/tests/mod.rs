mod directional_light;
mod point_light;
mod spot_light;

use glam::UVec2;

use crate::{load_sample, store_sample, Command};

use self::directional_light::directional_light;
use self::point_light::point_light;
use self::spot_light::spot_light;

#[derive(Clone, Debug)]
pub(crate) struct Options {
    pub(crate) cmd: Command,
    pub(crate) size: UVec2,
}

pub(crate) fn run_tests(options: Options) {
    let mut tests = Vec::new();
    tests.push(directional_light());
    tests.push(point_light());
    tests.push(spot_light());

    let mut result = TestResult {
        passed: 0,
        failed: 0,
    };

    let mut failures = Vec::new();

    println!("running {} tests", tests.len());
    for test in &mut tests {
        let buf = test.run(options.size);

        match options.cmd {
            Command::Generate => {
                store_sample(test.name, buf);
                println!("generate {} ... ok", test.name);
            }
            Command::Test => {
                let Some(sample) = load_sample(test.name) else {
                    result.failed += 1;
                    failures.push((test.name, FailureReason::NoSample));

                    continue;
                };

                if sample != buf {
                    result.failed += 1;
                    failures.push((test.name, FailureReason::Missmatch));

                    println!("test {} ... FAILED", test.name);
                } else {
                    result.passed += 1;
                    println!("test {} ... ok", test.name);
                }
            }
        }
    }

    let status = if result.failed == 0 { "ok" } else { "FAIL" };

    if !failures.is_empty() {
        println!("\nfailures:");
        for (name, reason) in failures {
            println!(
                "    {}: {}",
                name,
                match reason {
                    FailureReason::NoSample => "no sample available",
                    FailureReason::Missmatch => "content missmatch",
                }
            );
        }
    }
    println!("");

    println!(
        "test result: {}. {} passed; {} failed;",
        status, result.passed, result.failed
    );
}

#[derive(Copy, Clone, Debug)]
struct TestResult {
    passed: u64,
    failed: u64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum FailureReason {
    NoSample,
    Missmatch,
}

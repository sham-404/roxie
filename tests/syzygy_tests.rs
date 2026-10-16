use roxie::{
    board::{Board, init_ray_tables},
    evaluation::init_pesto_table,
    magics::init_magics,
    network::init_nn,
    search::init_lmr_table,
    syzygy::tbprobe::{init_tb, probe_dtz, probe_wdl},
    zobrist::init_zobrist,
};
use std::fs;
use std::sync::Once;

static INIT: Once = Once::new();

fn init_engine_tables() {
    INIT.call_once(|| {
        init_zobrist();
        init_pesto_table();
        init_magics();
        init_lmr_table();
        init_nn(true);
        init_ray_tables();
        init_tb("tests/syzygy");
    });
}

#[test]
fn verify_syzygy_wdl_dtz_from_csv() {
    init_engine_tables();

    let csv_path = "tests/3-4-5_tb_positions.csv";
    let csv_data = fs::read_to_string(csv_path)
        .unwrap_or_else(|_| panic!("Failed to read benchmark CSV at {}", csv_path));

    let mut passed = 0;
    let mut failed = 0;

    for (line_number, line) in csv_data.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("fen") || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split(',').collect();
        assert_eq!(
            parts.len(),
            3,
            "Malformed line {}: {}",
            line_number + 1,
            line
        );

        let fen = parts[0];
        let expected_wdl: i32 = parts[1].parse().expect("Failed to parse WDL");
        let expected_dtz: i32 = parts[2].parse().expect("Failed to parse DTZ");

        let mut board = Board::load_fen(fen);

        let mut wdl_success = 0;
        let wdl = probe_wdl(&mut board, &mut wdl_success);

        let mut dtz_success = 0;
        let dtz = probe_dtz(&mut board, &mut dtz_success);

        let wdl_pass = wdl_success > 0 && wdl == expected_wdl;
        let dtz_pass = dtz_success > 0 && dtz == expected_dtz;

        if wdl_pass && dtz_pass {
            passed += 1;
        } else {
            failed += 1;
            println!("FAILED [Line {}]: {}", line_number + 1, fen);
            if !wdl_pass {
                println!(
                    "   WDL -> Expected: {:2}, Got: {:2} (Success Flag: {})",
                    expected_wdl, wdl, wdl_success
                );
            }
            if !dtz_pass {
                println!(
                    "   DTZ -> Expected: {:3}, Got: {:3} (Success Flag: {})",
                    expected_dtz, dtz, dtz_success
                );
            }
        }
    }

    println!("\nSyzygy Verification Summary");
    println!("Passed: {}", passed);
    println!("Failed: {}", failed);

    assert_eq!(failed, 0, "One or more Syzygy tablebase checks failed.");
}

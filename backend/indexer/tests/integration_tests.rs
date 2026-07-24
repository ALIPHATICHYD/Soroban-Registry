/// Integration tests for the indexer service
/// These tests validate core functionality without requiring a real database
#[cfg(test)]
mod tests {
    use indexer::backoff::ExponentialBackoff;
    use indexer::detector::detect_contract_deployments;
    use indexer::rpc::Operation;
    use indexer::state::IndexerState;
    use serde_json::json;
    use shared::Network;

    #[test]
    fn test_exponential_backoff_sequence() {
        let mut backoff = ExponentialBackoff::new(1, 120);

        let d1 = backoff.on_failure("error1");
        assert_eq!(d1.as_secs(), 1);

        let d2 = backoff.on_failure("error2");
        assert_eq!(d2.as_secs(), 2);

        let d3 = backoff.on_failure("error3");
        assert_eq!(d3.as_secs(), 4);

        let d4 = backoff.on_failure("error4");
        assert_eq!(d4.as_secs(), 8);
    }

    #[test]
    fn test_backoff_max_interval_capped() {
        let mut backoff = ExponentialBackoff::new(1, 10);

        for _ in 0..10 {
            backoff.on_failure("test");
        }

        assert_eq!(backoff.interval_secs(), 10);
    }

    #[test]
    fn test_backoff_reset_on_success() {
        let mut backoff = ExponentialBackoff::new(1, 60);

        backoff.on_failure("error1");
        backoff.on_failure("error2");
        assert_eq!(backoff.attempts(), 2);

        backoff.on_success();
        assert_eq!(backoff.attempts(), 0);
        assert_eq!(backoff.interval_secs(), 1);
    }

    #[test]
    fn test_detect_contract_deployment() {
        let contract_id = "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4".to_string();
        let deployer = "GBRPYHIL2CI3WHZDTOOQFC6EB4RRJC3D5NZ4FJHSVOBXUXVLCJGXI2V".to_string();

        let ops = vec![Operation {
            id: "op123".to_string(),
            tx_id: "tx456".to_string(),
            type_code: 110, // createContract
            type_name: "createContract".to_string(),
            body: json!({
                "contract": contract_id.clone(),
                "source_account": deployer.clone(),
            }),
        }];

        let deployments = detect_contract_deployments(&ops, 100, "2023-10-27T10:00:00Z");
        assert_eq!(deployments.len(), 1);
        assert_eq!(deployments[0].contract_id, contract_id);
        assert_eq!(deployments[0].deployer, deployer);
        assert_eq!(deployments[0].ledger_sequence, 100);
    }

    #[test]
    fn test_detect_ignores_non_contract_operations() {
        let ops = vec![
            Operation {
                id: "op1".to_string(),
                tx_id: "tx1".to_string(),
                type_code: 1, // payment
                type_name: "payment".to_string(),
                body: json!({}),
            },
            Operation {
                id: "op2".to_string(),
                tx_id: "tx2".to_string(),
                type_code: 4, // path_payment
                type_name: "path_payment".to_string(),
                body: json!({}),
            },
        ];

        let deployments = detect_contract_deployments(&ops, 100, "2023-10-27T10:00:00Z");
        assert_eq!(deployments.len(), 0);
    }

    #[test]
    fn test_state_next_ledger_to_process() {
        let state = IndexerState {
            network: Network::Testnet,
            last_indexed_ledger_height: 100,
            last_indexed_ledger_hash: Some("hash1".to_string()),
            last_checkpoint_ledger_height: 100,
            consecutive_failures: 0,
        };

        assert_eq!(state.next_ledger_to_process(), 101);
    }

    #[test]
    fn test_state_failure_tracking() {
        let mut state = IndexerState {
            network: Network::Testnet,
            last_indexed_ledger_height: 100,
            last_indexed_ledger_hash: Some("hash1".to_string()),
            last_checkpoint_ledger_height: 100,
            consecutive_failures: 0,
        };

        assert_eq!(state.consecutive_failures, 0);

        state.record_failure();
        assert_eq!(state.consecutive_failures, 1);

        state.record_failure();
        assert_eq!(state.consecutive_failures, 2);

        state.clear_failures();
        assert_eq!(state.consecutive_failures, 0);
    }

    #[test]
    fn test_state_checkpoint_update() {
        let mut state = IndexerState {
            network: Network::Testnet,
            last_indexed_ledger_height: 100,
            last_indexed_ledger_hash: Some("hash1".to_string()),
            last_checkpoint_ledger_height: 50,
            consecutive_failures: 0,
        };

        state.update_checkpoint(100);
        assert_eq!(state.last_checkpoint_ledger_height, 100);
    }

    #[test]
    fn test_multiple_contract_detections() {
        let c1 = "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4".to_string();
        let c2 = "CBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBSC4".to_string();

        let ops = vec![
            Operation {
                id: "op1".to_string(),
                tx_id: "tx1".to_string(),
                type_code: 110,
                type_name: "createContract".to_string(),
                body: json!({
                    "contract": c1.clone(),
                    "source_account": "G111111111111111111111111111111111111111111111111111WHSRQ",
                }),
            },
            Operation {
                id: "op2".to_string(),
                tx_id: "tx2".to_string(),
                type_code: 110,
                type_name: "createContract".to_string(),
                body: json!({
                    "contract": c2.clone(),
                    "source_account": "G222222222222222222222222222222222222222222222222222WHSRQ",
                }),
            },
        ];

        let deployments = detect_contract_deployments(&ops, 100, "2023-10-27T10:00:00Z");
        assert_eq!(deployments.len(), 2);
        assert_eq!(deployments[0].contract_id, c1);
        assert_eq!(deployments[1].contract_id, c2);
    }

    #[test]
    fn test_invalid_contract_id_rejected() {
        let ops = vec![Operation {
            id: "op1".to_string(),
            tx_id: "tx1".to_string(),
            type_code: 110,
            type_name: "createContract".to_string(),
            body: json!({
                "contract": "INVALID",
                "source_account": "GBRPYHIL2CI3WHZDTOOQFC6EB4RRJC3D5NZ4FJHSVOBXUXVLCJGXI2V",
            }),
        }];

        let deployments = detect_contract_deployments(&ops, 100, "2023-10-27T10:00:00Z");
        assert_eq!(deployments.len(), 0);
    }

    #[tokio::test]
    async fn test_backoff_execute_immediate_success() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let backoff = ExponentialBackoff::new(1, 60);
        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = call_count.clone();

        let result = indexer::backoff::execute_with_backoff(backoff, 5, move || {
            let count = call_count_clone.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Ok::<i32, String>(42)
            }
        })
        .await;

        assert_eq!(result, Ok(42));
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_backoff_execute_retry_then_success() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let backoff = ExponentialBackoff::new(1, 60);
        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = call_count.clone();

        let result = indexer::backoff::execute_with_backoff(backoff, 5, move || {
            let count = call_count_clone.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                let current = count.load(Ordering::SeqCst);
                if current < 3 {
                    Err::<i32, _>("temporary failure".to_string())
                } else {
                    Ok(42)
                }
            }
        })
        .await;

        assert_eq!(result, Ok(42));
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_backoff_execute_max_attempts_reached() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let backoff = ExponentialBackoff::new(1, 60);
        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = call_count.clone();

        let result = indexer::backoff::execute_with_backoff(backoff, 3, move || {
            let count = call_count_clone.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Err::<i32, _>("persistent failure".to_string())
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }
}

use oreslang_format::{format_source, is_formatted};

fn check(input: &str, expected: &str) {
    let got = format_source(input).unwrap();
    assert_eq!(got, expected);
    assert_eq!(format_source(&got).unwrap(), got);
    assert!(is_formatted(&got).unwrap());
    assert!(!got.contains('\t'));
}

#[test]
fn orestra_driver_corpus_has_canonical_callable_parameters() {
    check(
        r#"define module cni_validation as
  pub fnc valid_mtu(int mtu, int minimum_mtu, int maximum_mtu): bool {
    return mtu >= minimum_mtu & mtu <= maximum_mtu & minimum_mtu > 0;
  }
  pub fnc may_attach(bool authorized_namespace, bool policy_accepted,
                     bool sandbox_identity_valid): bool {
    return authorized_namespace & policy_accepted & sandbox_identity_valid;
  }
end
"#,
        r#"define module cni_validation as
  pub fnc valid_mtu(int mtu, int minimum_mtu, int maximum_mtu) -> bool {
    return mtu >= minimum_mtu & mtu <= maximum_mtu & minimum_mtu > 0;
  }
  pub fnc may_attach(
    bool authorized_namespace,
    bool policy_accepted,
    bool sandbox_identity_valid
  ) -> bool {
    return authorized_namespace & policy_accepted & sandbox_identity_valid;
  }
end
"#,
    );

    check(
        r#"define module csi_validation as
  pub fnc may_attach(bool volume_exists, bool node_authorized,
                     bool volume_fenced, bool exclusive_in_use): bool {
    return volume_exists & node_authorized & volume_fenced & !exclusive_in_use;
  }
  pub fnc can_detach(bool unmounted, bool no_active_users): bool {
    return unmounted & no_active_users;
  }
end
"#,
        r#"define module csi_validation as
  pub fnc may_attach(
    bool volume_exists,
    bool node_authorized,
    bool volume_fenced,
    bool exclusive_in_use
  ) -> bool {
    return volume_exists & node_authorized & volume_fenced & !exclusive_in_use;
  }
  pub fnc can_detach(bool unmounted, bool no_active_users) -> bool {
    return unmounted & no_active_users;
  }
end
"#,
    );

    check(
        r#"define module driver_registry as
  // Numeric ABI versions are negotiated independently of plugin versions.
  pub fnc abi_compatible(int minimum_supported, int maximum_supported,
                         int requested_abi): bool {
    return minimum_supported > 0 &
         maximum_supported >= minimum_supported &
         requested_abi >= minimum_supported &
         requested_abi <= maximum_supported;
  }
  pub fnc allow_plugin(bool signature_verified, bool tenant_allowed,
                       bool sandboxed, bool abi_compatible): bool {
    return signature_verified & tenant_allowed & sandboxed & abi_compatible;
  }
end
"#,
        r#"define module driver_registry as
  // Numeric ABI versions are negotiated independently of plugin versions.
  pub fnc abi_compatible(int minimum_supported, int maximum_supported, int requested_abi) -> bool {
    return minimum_supported > 0 &
    maximum_supported >= minimum_supported &
    requested_abi >= minimum_supported &
    requested_abi <= maximum_supported;
  }
  pub fnc allow_plugin(
    bool signature_verified,
    bool tenant_allowed,
    bool sandboxed,
    bool abi_compatible
  ) -> bool {
    return signature_verified & tenant_allowed & sandboxed & abi_compatible;
  }
end
"#,
    );

    check(
        r#"// Pure node-runtime lifecycle guards. Host process execution lives behind an
// authenticated driver boundary; these helpers do not execute commands.
define module oci_lifecycle as
  pub fnc start_authorized(int current_agent_epoch, int claimed_agent_epoch,
                           int requested_generation, int observed_generation,
                           bool same_workload_uid, bool deleting): bool {
    if current_agent_epoch <= 0 ||
     claimed_agent_epoch != current_agent_epoch; do return false; fi
    if requested_generation <= 0 || observed_generation < 0; do return false; fi
    if requested_generation < observed_generation; do return false; fi
    return same_workload_uid && !deleting;
  }

  pub fnc stop_authorized(int current_agent_epoch, int claimed_agent_epoch,
                          bool sandbox_identity_valid): bool {
    return current_agent_epoch > 0 &&
         claimed_agent_epoch == current_agent_epoch &&
         sandbox_identity_valid;
  }

  pub fnc release_resources(bool stop_requested,
                            bool process_absent_confirmed): bool {
    return stop_requested && process_absent_confirmed;
  }
end
"#,
        r#"// Pure node-runtime lifecycle guards. Host process execution lives behind an
// authenticated driver boundary; these helpers do not execute commands.
define module oci_lifecycle as
  pub fnc start_authorized(
    int current_agent_epoch,
    int claimed_agent_epoch,
    int requested_generation,
    int observed_generation,
    bool same_workload_uid,
    bool deleting
  ) -> bool {
    if current_agent_epoch <= 0 || claimed_agent_epoch != current_agent_epoch; then
      return false;
    fi
    if requested_generation <= 0 || observed_generation < 0; then
      return false;
    fi
    if requested_generation < observed_generation; then
      return false;
    fi
    return same_workload_uid && !deleting;
  }

  pub fnc stop_authorized(
    int current_agent_epoch,
    int claimed_agent_epoch,
    bool sandbox_identity_valid
  ) -> bool {
    return current_agent_epoch > 0 &&
    claimed_agent_epoch == current_agent_epoch &&
    sandbox_identity_valid;
  }

  pub fnc release_resources(bool stop_requested, bool process_absent_confirmed) -> bool {
    return stop_requested && process_absent_confirmed;
  }
end
"#,
    );

    check(
        r#"// Pure validation for OCI runtime resources, not the runtime invocation itself.
define module oci_validation as
  pub fnc valid_limits(int cpu_millis, int memory_bytes, int pids_limit): bool {
    return cpu_millis > 0 & memory_bytes > 0 & pids_limit > 0;
  }
  pub fnc may_reuse_container(int requested_generation, int observed_generation,
                              bool same_workload_uid, bool runtime_alive): bool {
    return requested_generation == observed_generation &
         requested_generation > 0 & same_workload_uid & runtime_alive;
  }
end
"#,
        r#"// Pure validation for OCI runtime resources, not the runtime invocation itself.
define module oci_validation as
  pub fnc valid_limits(int cpu_millis, int memory_bytes, int pids_limit) -> bool {
    return cpu_millis > 0 & memory_bytes > 0 & pids_limit > 0;
  }
  pub fnc may_reuse_container(
    int requested_generation,
    int observed_generation,
    bool same_workload_uid,
    bool runtime_alive
  ) -> bool {
    return requested_generation == observed_generation &
    requested_generation > 0 & same_workload_uid & runtime_alive;
  }
end
"#,
    );
}

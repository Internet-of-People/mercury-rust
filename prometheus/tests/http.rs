use structopt::StructOpt;

use claims::api::Api;
use prometheus::daemon::Daemon;
use prometheus::http::client::ApiHttpClient;
use prometheus::options::Options;

#[test]
fn test_http_api() {
    let tempdir = tempfile::tempdir().unwrap().into_path().into_os_string().into_string().unwrap();
    let args = vec!["dummy_test_binary_name", "--config-dir", &tempdir];
    let options = Options::from_iter_safe(args).unwrap();
    let url = format!("http://{}", options.listen_on);

    log4rs::init_file(&options.logger_config, Default::default()).unwrap();

    let mut daemon = Daemon::start(options).unwrap();
    let mut api = ApiHttpClient::new(&url);

    // TODO get this demo phrase from a single constant in some crate
    api.restore_vault("include pear escape sail spy orange cute despair witness trouble sleep torch wire burst unable brass expose fiction drift clock duck oxygen aerobic already".to_owned()).unwrap();
    {
        assert!(api.list_vault_records().unwrap().is_empty());
    }
    let counts = api.restore_all_profiles().unwrap();
    {
        // TODO somehow test this with a previously saved profile
        assert_eq!(counts.try_count, did::vault::GAP);
        assert_eq!(counts.restore_count, 0);

        let active_opt = api.get_active_profile().unwrap();
        assert_eq!(active_opt, None);
    }

    let first_id = api.create_profile(None).unwrap();
    let second_id = api.create_profile(Some("SecondTestProfileOriginal".to_owned())).unwrap();
    {
        let profiles = api.list_vault_records().unwrap();
        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[0].id(), first_id);
        assert_eq!(profiles[1].id(), second_id);

        let first_profile = api.get_vault_record(Some(first_id.clone())).unwrap();
        assert_eq!(profiles[0], first_profile);
        assert_eq!(first_profile.label(), "Logical Marvel");

        let second_profile = api.get_vault_record(Some(second_id.clone())).unwrap();
        assert_eq!(profiles[1], second_profile);
        assert_eq!(second_profile.label(), "SecondTestProfileOriginal");

        let active_profile = api.get_vault_record(None).unwrap();
        assert_eq!(second_profile, active_profile);

        let active_opt = api.get_active_profile().unwrap();
        assert_eq!(active_opt, Some(second_id.clone()));
    }

    api.set_profile_label(Some(second_id.clone()), "SecondTestProfile".to_owned()).unwrap();
    {
        let second_profile = api.get_vault_record(Some(second_id.clone())).unwrap();
        assert_eq!(second_profile.id(), second_id);
        assert_eq!(second_profile.label(), "SecondTestProfile");
    }

    api.set_active_profile(&first_id).unwrap();
    let active_profile = api.get_active_profile().unwrap();
    {
        assert_eq!(active_profile, Some(first_id.clone()));
    }

    let schemas = api.claim_schemas().unwrap();
    {
        assert_eq!(schemas.iter().count(), 3);
    }

    daemon.stop().unwrap();
    daemon.join().unwrap();
}

use structopt::StructOpt;

use claims::api::Api;
use prometheus::daemon::Daemon;
use prometheus::http::client::ApiHttpClient;
use prometheus::options::Options;

#[test]
fn test_http_api() {
    let tempdir = tempfile::tempdir().unwrap().into_path().into_os_string().into_string().unwrap();
    let args = vec!["dummy_test_binary_name", "--config_dir", &tempdir];
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

    let profile_id = api.create_profile(Some("FirstTestProfile".to_owned())).unwrap();
    {
        let profiles = api.list_vault_records().unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].id(), profile_id);
    }

    daemon.stop().unwrap();
    daemon.join().unwrap();
}

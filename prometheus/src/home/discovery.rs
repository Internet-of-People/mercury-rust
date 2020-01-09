use std::collections::HashMap;
use std::time::Duration;

use failure::{bail, Fallible};
use log::*;
use multiaddr::Multiaddr;

use mercury_home_protocol::{HomeFacet, Profile, ProfileFacets, ProfileId};

pub struct KnownHomeNode {
    // TODO should we store HomeFacet or the whole Profile here?
    pub profile: Profile,
    pub latency: Option<Duration>,
}

impl KnownHomeNode {
    pub fn addrs(&self) -> Vec<Multiaddr> {
        match self.profile.to_home() {
            None => vec![],
            Some(home_facet) => home_facet.addrs.to_owned(),
        }
    }
}

pub struct HomeNodeCrawler {
    pub home_profiles: HashMap<ProfileId, KnownHomeNode>,
}

impl Default for HomeNodeCrawler {
    fn default() -> Self {
        let mut result = Self { home_profiles: Default::default() };
        let facet = HomeFacet::new(vec!["/ip4/127.0.0.1/tcp/2077".parse().unwrap()], vec![]);
        let attributes = facet.to_attribute_map();
        result
            .add(&Profile::new(
                "pez7aYuvoDPM5i7xedjwjsWaFVzL3qRKPv4sBLv3E3pAGi6".parse().unwrap(),
                1,
                vec![],
                attributes,
            ))
            .unwrap();
        result
    }
}

impl HomeNodeCrawler {
    pub fn add(&mut self, home: &Profile) -> Fallible<()> {
        let home_facet = match home.to_home() {
            None => bail!("Not a profile of a home node"),
            Some(facet) => facet,
        };
        self.home_profiles
            .entry(home.id())
            .and_modify(|p| {
                if p.profile.version() >= home.version() {
                    info!("Ignored older version {} of profile {}", home.version(), home.id());
                } else {
                    p.profile = home.to_owned();
                }
            })
            .or_insert(KnownHomeNode { profile: home.to_owned(), latency: None });
        Ok(())
    }

    pub fn iter(&self) -> impl Iterator<Item = &KnownHomeNode> {
        self.home_profiles.values()
    }
}

use riven::consts::{PlatformRoute, RegionalRoute};
use sfg_bot::riot::client::RiotClient;

mod platform_for_region {
    use super::*;

    #[test]
    fn euw1_returns_euw1() {
        assert!(matches!(
            RiotClient::platform_for_region("euw1"),
            PlatformRoute::EUW1
        ));
    }

    #[test]
    fn euw_returns_euw1() {
        assert!(matches!(
            RiotClient::platform_for_region("euw"),
            PlatformRoute::EUW1
        ));
    }

    #[test]
    fn euw_uppercase_returns_euw1() {
        assert!(matches!(
            RiotClient::platform_for_region("EUW"),
            PlatformRoute::EUW1
        ));
    }

    #[test]
    fn na1_returns_na1() {
        assert!(matches!(
            RiotClient::platform_for_region("na1"),
            PlatformRoute::NA1
        ));
    }

    #[test]
    fn na_returns_na1() {
        assert!(matches!(
            RiotClient::platform_for_region("na"),
            PlatformRoute::NA1
        ));
    }

    #[test]
    fn kr_returns_kr() {
        assert!(matches!(
            RiotClient::platform_for_region("kr"),
            PlatformRoute::KR
        ));
    }

    #[test]
    fn jp1_returns_jp1() {
        assert!(matches!(
            RiotClient::platform_for_region("jp1"),
            PlatformRoute::JP1
        ));
    }

    #[test]
    fn jp_returns_jp1() {
        assert!(matches!(
            RiotClient::platform_for_region("jp"),
            PlatformRoute::JP1
        ));
    }

    #[test]
    fn br1_returns_br1() {
        assert!(matches!(
            RiotClient::platform_for_region("br1"),
            PlatformRoute::BR1
        ));
    }

    #[test]
    fn br_returns_br1() {
        assert!(matches!(
            RiotClient::platform_for_region("br"),
            PlatformRoute::BR1
        ));
    }

    #[test]
    fn eun1_returns_eun1() {
        assert!(matches!(
            RiotClient::platform_for_region("eun1"),
            PlatformRoute::EUN1
        ));
    }

    #[test]
    fn eune_returns_eun1() {
        assert!(matches!(
            RiotClient::platform_for_region("eune"),
            PlatformRoute::EUN1
        ));
    }

    #[test]
    fn la1_returns_la1() {
        assert!(matches!(
            RiotClient::platform_for_region("la1"),
            PlatformRoute::LA1
        ));
    }

    #[test]
    fn lan_returns_la1() {
        assert!(matches!(
            RiotClient::platform_for_region("lan"),
            PlatformRoute::LA1
        ));
    }

    #[test]
    fn la2_returns_la2() {
        assert!(matches!(
            RiotClient::platform_for_region("la2"),
            PlatformRoute::LA2
        ));
    }

    #[test]
    fn las_returns_la2() {
        assert!(matches!(
            RiotClient::platform_for_region("las"),
            PlatformRoute::LA2
        ));
    }

    #[test]
    fn oc1_returns_oc1() {
        assert!(matches!(
            RiotClient::platform_for_region("oc1"),
            PlatformRoute::OC1
        ));
    }

    #[test]
    fn oce_returns_oc1() {
        assert!(matches!(
            RiotClient::platform_for_region("oce"),
            PlatformRoute::OC1
        ));
    }

    #[test]
    fn tr1_returns_tr1() {
        assert!(matches!(
            RiotClient::platform_for_region("tr1"),
            PlatformRoute::TR1
        ));
    }

    #[test]
    fn tr_returns_tr1() {
        assert!(matches!(
            RiotClient::platform_for_region("tr"),
            PlatformRoute::TR1
        ));
    }

    #[test]
    fn ru_returns_ru() {
        assert!(matches!(
            RiotClient::platform_for_region("ru"),
            PlatformRoute::RU
        ));
    }

    #[test]
    fn unknown_region_defaults_to_euw1() {
        assert!(matches!(
            RiotClient::platform_for_region("unknown"),
            PlatformRoute::EUW1
        ));
    }

    #[test]
    fn empty_string_defaults_to_euw1() {
        assert!(matches!(
            RiotClient::platform_for_region(""),
            PlatformRoute::EUW1
        ));
    }

    #[test]
    fn mixed_case_works() {
        assert!(matches!(
            RiotClient::platform_for_region("EuW1"),
            PlatformRoute::EUW1
        ));
    }
}

mod regional_for_region {
    use super::*;

    #[test]
    fn euw1_returns_europe() {
        assert!(matches!(
            RiotClient::regional_for_region("euw1"),
            RegionalRoute::EUROPE
        ));
    }

    #[test]
    fn euw_returns_europe() {
        assert!(matches!(
            RiotClient::regional_for_region("euw"),
            RegionalRoute::EUROPE
        ));
    }

    #[test]
    fn eune_returns_europe() {
        assert!(matches!(
            RiotClient::regional_for_region("eune"),
            RegionalRoute::EUROPE
        ));
    }

    #[test]
    fn eun1_returns_europe() {
        assert!(matches!(
            RiotClient::regional_for_region("eun1"),
            RegionalRoute::EUROPE
        ));
    }

    #[test]
    fn tr_returns_europe() {
        assert!(matches!(
            RiotClient::regional_for_region("tr"),
            RegionalRoute::EUROPE
        ));
    }

    #[test]
    fn ru_returns_europe() {
        assert!(matches!(
            RiotClient::regional_for_region("ru"),
            RegionalRoute::EUROPE
        ));
    }

    #[test]
    fn na1_returns_americas() {
        assert!(matches!(
            RiotClient::regional_for_region("na1"),
            RegionalRoute::AMERICAS
        ));
    }

    #[test]
    fn na_returns_americas() {
        assert!(matches!(
            RiotClient::regional_for_region("na"),
            RegionalRoute::AMERICAS
        ));
    }

    #[test]
    fn br1_returns_americas() {
        assert!(matches!(
            RiotClient::regional_for_region("br1"),
            RegionalRoute::AMERICAS
        ));
    }

    #[test]
    fn br_returns_americas() {
        assert!(matches!(
            RiotClient::regional_for_region("br"),
            RegionalRoute::AMERICAS
        ));
    }

    #[test]
    fn la1_returns_americas() {
        assert!(matches!(
            RiotClient::regional_for_region("la1"),
            RegionalRoute::AMERICAS
        ));
    }

    #[test]
    fn lan_returns_americas() {
        assert!(matches!(
            RiotClient::regional_for_region("lan"),
            RegionalRoute::AMERICAS
        ));
    }

    #[test]
    fn la2_returns_americas() {
        assert!(matches!(
            RiotClient::regional_for_region("la2"),
            RegionalRoute::AMERICAS
        ));
    }

    #[test]
    fn las_returns_americas() {
        assert!(matches!(
            RiotClient::regional_for_region("las"),
            RegionalRoute::AMERICAS
        ));
    }

    #[test]
    fn oc1_returns_americas() {
        assert!(matches!(
            RiotClient::regional_for_region("oc1"),
            RegionalRoute::AMERICAS
        ));
    }

    #[test]
    fn oce_returns_americas() {
        assert!(matches!(
            RiotClient::regional_for_region("oce"),
            RegionalRoute::AMERICAS
        ));
    }

    #[test]
    fn kr_returns_asia() {
        assert!(matches!(
            RiotClient::regional_for_region("kr"),
            RegionalRoute::ASIA
        ));
    }

    #[test]
    fn jp1_returns_asia() {
        assert!(matches!(
            RiotClient::regional_for_region("jp1"),
            RegionalRoute::ASIA
        ));
    }

    #[test]
    fn jp_returns_asia() {
        assert!(matches!(
            RiotClient::regional_for_region("jp"),
            RegionalRoute::ASIA
        ));
    }

    #[test]
    fn sg2_returns_sea() {
        assert!(matches!(
            RiotClient::regional_for_region("sg2"),
            RegionalRoute::SEA
        ));
    }

    #[test]
    fn sg_returns_sea() {
        assert!(matches!(
            RiotClient::regional_for_region("sg"),
            RegionalRoute::SEA
        ));
    }

    #[test]
    fn tw2_returns_sea() {
        assert!(matches!(
            RiotClient::regional_for_region("tw2"),
            RegionalRoute::SEA
        ));
    }

    #[test]
    fn vn2_returns_sea() {
        assert!(matches!(
            RiotClient::regional_for_region("vn2"),
            RegionalRoute::SEA
        ));
    }

    #[test]
    fn unknown_region_defaults_to_europe() {
        assert!(matches!(
            RiotClient::regional_for_region("unknown"),
            RegionalRoute::EUROPE
        ));
    }

    #[test]
    fn empty_string_defaults_to_europe() {
        assert!(matches!(
            RiotClient::regional_for_region(""),
            RegionalRoute::EUROPE
        ));
    }

    #[test]
    fn mixed_case_works() {
        assert!(matches!(
            RiotClient::regional_for_region("EuW1"),
            RegionalRoute::EUROPE
        ));
    }
}

mod riot_client_new {
    use super::*;

    #[test]
    fn can_create_client() {
        let _client = RiotClient::new("test-api-key");
    }

    #[test]
    fn can_create_client_with_empty_key() {
        let _client = RiotClient::new("");
    }
}

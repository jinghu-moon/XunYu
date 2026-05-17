use std::collections::{HashMap, HashSet};

use comfy_table::{Attribute, Cell, Color, Table};
use console::Term;
use dialoguer::{Confirm, MultiSelect, theme::ColorfulTheme};

use crate::xun_core::port_cmd::{KillCmd, PortsListArgs as PortsCmd};
use crate::xun_core::proc_cmd::{PkillCmd, PsListArgs as PsCmd};
use crate::model::{ListFormat, parse_list_format};
use crate::output::{CliError, CliResult};
use crate::output::{apply_pretty_table_style, can_interact, prefer_table_output, print_table};
use crate::ports::{PortInfo, Protocol, list_tcp_listeners, list_udp_endpoints, terminate_pid};
use crate::proc::{self, KillResult, ProcInfo};

mod common;
mod kill;
mod process;
mod query;
mod render;

pub(crate) use kill::cmd_kill;
pub(crate) use process::{cmd_pkill, cmd_ps};
pub(crate) use query::cmd_ports;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_dev_port_matches_expected_ranges() {
        assert!(common::is_dev_port(3000));
        assert!(common::is_dev_port(3999));
        assert!(common::is_dev_port(5000));
        assert!(common::is_dev_port(5999));
        assert!(common::is_dev_port(8000));
        assert!(common::is_dev_port(8999));
        assert!(common::is_dev_port(4173));
        assert!(common::is_dev_port(5173));

        assert!(!common::is_dev_port(2999));
        assert!(!common::is_dev_port(4000));
        assert!(!common::is_dev_port(6000));
        assert!(!common::is_dev_port(9000));
    }

    #[test]
    fn parse_range_parses_and_normalizes() {
        assert_eq!(common::parse_range("3000-4000"), Some((3000, 4000)));
        assert_eq!(common::parse_range("4000-3000"), Some((3000, 4000)));
        assert_eq!(common::parse_range(" 3000 - 4000 "), Some((3000, 4000)));
        assert_eq!(common::parse_range("3000"), None);
        assert_eq!(common::parse_range("a-b"), None);
        assert_eq!(common::parse_range("1-2-3"), None);
    }

    #[test]
    fn trunc_short_strings_are_unchanged_and_long_strings_keep_suffix() {
        assert_eq!(common::trunc("abc", 10), "abc");
        assert_eq!(common::trunc("0123456789", 6), "...789");
    }
}

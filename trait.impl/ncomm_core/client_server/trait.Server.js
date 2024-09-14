(function() {var implementors = {
"ncomm":[],
"ncomm_clients_and_servers":[["impl&lt;Req, Res, K: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/hash/trait.Hash.html\" title=\"trait core::hash::Hash\">Hash</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/cmp/trait.Eq.html\" title=\"trait core::cmp::Eq\">Eq</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a>&gt; <a class=\"trait\" href=\"ncomm_core/client_server/trait.Server.html\" title=\"trait ncomm_core::client_server::Server\">Server</a> for <a class=\"struct\" href=\"ncomm_clients_and_servers/local/struct.LocalServer.html\" title=\"struct ncomm_clients_and_servers::local::LocalServer\">LocalServer</a>&lt;Req, Res, K&gt;"],["impl&lt;Req, Res, Serial, Err, const BUFFER_SIZE: <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.81.0/std/primitive.usize.html\">usize</a>&gt; <a class=\"trait\" href=\"ncomm_core/client_server/trait.Server.html\" title=\"trait ncomm_core::client_server::Server\">Server</a> for <a class=\"struct\" href=\"ncomm_clients_and_servers/serial/struct.SerialServer.html\" title=\"struct ncomm_clients_and_servers::serial::SerialServer\">SerialServer</a>&lt;Req, Res, Serial, Err, BUFFER_SIZE&gt;<div class=\"where\">where\n    Req: <a class=\"trait\" href=\"ncomm_utils/packing/trait.Packable.html\" title=\"trait ncomm_utils::packing::Packable\">Packable</a>,\n    Res: <a class=\"trait\" href=\"ncomm_utils/packing/trait.Packable.html\" title=\"trait ncomm_utils::packing::Packable\">Packable</a>,\n    Serial: ReadReady&lt;Error = Err&gt; + Read&lt;Error = Err&gt; + Write&lt;Error = Err&gt;,\n    Err: Error,</div>"],["impl&lt;Req: <a class=\"trait\" href=\"ncomm_utils/packing/trait.Packable.html\" title=\"trait ncomm_utils::packing::Packable\">Packable</a>, Res: <a class=\"trait\" href=\"ncomm_utils/packing/trait.Packable.html\" title=\"trait ncomm_utils::packing::Packable\">Packable</a>, K: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/cmp/trait.Eq.html\" title=\"trait core::cmp::Eq\">Eq</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a>&gt; <a class=\"trait\" href=\"ncomm_core/client_server/trait.Server.html\" title=\"trait ncomm_core::client_server::Server\">Server</a> for <a class=\"struct\" href=\"ncomm_clients_and_servers/udp/struct.UdpServer.html\" title=\"struct ncomm_clients_and_servers::udp::UdpServer\">UdpServer</a>&lt;Req, Res, K&gt;"]]
};if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()
use argparse::{ArgumentParser, Store, StoreTrue};
use lazy_static::lazy_static;

pub struct Args {
    pub server: bool,
    pub client: bool,

    pub listen: String,
    pub upstream: String,

    pub no_tls: bool,
    pub identity: String,
    pub password: String,
}

lazy_static! {
    pub static ref ARGS: Args = {
        let mut args = Args {
            server: false,
            client: false,
            listen: String::new(),
            upstream: String::new(),

            no_tls: false,
            identity: String::new(),
            password: String::new(),
        };

        {
            let mut parser = ArgumentParser::new();

            parser.set_description("DNS over HTTPS proxy");

            parser.refer(&mut args.server)
                .add_option(&["--server"], StoreTrue,
                            "Accept HTTPS request and forward to DNS server");

            parser.refer(&mut args.client)
                .add_option(&["--client"], StoreTrue,
                            "Accept DNS requests and forward to DoH server");

            parser.refer(&mut args.listen)
                .required()
                .add_option(&["-l", "--listen"], Store,
                            "Listen address");

            parser.refer(&mut args.upstream)
                .required()
                .add_option(&["-u", "--upstream"], Store,
                            "Upstream address");

            parser.refer(&mut args.no_tls)
                .add_option(&["--no-https"], StoreTrue,
                            "Disable HTTPS and accept plain HTTP requests");

            parser.refer(&mut args.identity)
                .add_option(&["-i", "--identity"], Store,
                            "The path of TLS identity in PKCS#12 format");

            parser.refer(&mut args.password)
                .add_option(&["-p", "--password"], Store,
                            "The password of TLS identity");

            parser.parse_args_or_exit();
        }

        args
    };
}

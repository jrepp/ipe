# ServiceNow integration stand-in

`python3 testing/integration/servicenow_standin.py` starts a loopback HTTPS server,
generates a temporary certificate, and prints its endpoint and CA certificate path.
Stop with Ctrl-C. It requires Python 3 and `openssl`, with no Python packages.
Use `--directory PATH` to retain the generated fixture certificate and key.

The stand-in requires the synthetic Bearer token declared as `SECRET` in its source;
the demo and acceptance runner configure that token automatically. Do not configure
a production credential for this fixture.

The stand-in provides synthetic `sys_db_object`, `sys_dictionary`, and `sys_choice`
Table API envelopes. It deliberately includes hostile labels, internal-looking
links, and raw source identities to test disclosure boundaries. These are synthetic
markers, never real credentials or topology. It does not emulate ServiceNow ACLs,
OAuth, business rules, or application licensing; successful tests prove the local
contract, not compatibility with a particular ServiceNow instance.

`--mode` selects success, empty, truncated, wrong_shape, denied, unauthorized,
failure, unsupported, redirect, malformed, or oversize. The acceptance runner
changes modes in-process and records upstream requests to inspect their bounds.
The standalone URL and fixture certificate are operator configuration; they must
never appear in agent discovery responses.

local p = os.getenv("PKTGEN_HOME") .. "/"
package.path = package.path ..";" .. p .. "?.lua;" .. p .. "test/?.lua;" .. p .. "app/?.lua;"

local results_dir = "tmp/results/"

require "Pktgen"
local inspect = require('inspect');

local pktSizes = { 64, 128, 256, 512, 1024, 1280, 1518 };

local pkt_sz_env = tonumber(os.getenv("TEST_STRESS_SZ"))
local this_iter = tonumber(os.getenv("TEST_STRESS_ITER"))
local target_rate = tonumber(os.getenv("TEST_STRESS_RATE"))
local target_rate_mbps = os.getenv("TEST_STRESS_RATE_DISPLAY")
local core_ct = os.getenv("TEST_STRESS_CORE_CT")
local dst_mac = os.getenv("TEST_STRESS_DST_MAC")
local expt_suffix = os.getenv("TEST_STRESS_SUFFIX")

function run_latency(pkt_sz, iter_no)
	-- latency
	pktgen.set(0, "rate", target_rate);

	pktgen.latsampler_params(0, "simple", 40000, 4000, results_dir .. string.format("l-%dB-%sM-%sC-%d%s.dat", pkt_sz, target_rate_mbps, core_ct, iter_no, expt_suffix))
	pktgen.latsampler("0", "on")
	pktgen.start(0)
	pktgen.delay(11000);
	pktgen.latsampler("0", "off")
	pktgen.stop(0)

	pktgen.delay(1000);

	local tput_file_name = results_dir .. string.format("t-%dB-%sM-%sC-%d%s.dat", pkt_sz, target_rate_mbps, core_ct, iter_no, expt_suffix)
	local tput_file = io.open(tput_file_name, "w")
	tput_file:write(inspect(pktgen.portStats(0, "port")))
	tput_file:close()

	pktgen.clr()
end

function run_expt(pkt_sz, iter_no)
	pktgen.clr()
	pktgen.set(0, "size", pkt_sz);
	if dst_mac then pktgen.set_mac(0, "dst", dst_mac); end
	run_latency(pkt_sz, iter_no)
	pktgen.delay(10000);
end

function main()
	pktgen.screen("off");
	-- local pkt_sz = pktSizes[pkt_sz_idx + 1]
	printf("Starting %dB, iter %d\n", pkt_sz_env, this_iter)
	run_expt(pkt_sz_env, this_iter)
	pktgen.quit();
end

main()

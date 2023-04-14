import pathlib

results_dir = "SUMMARY/"
out_dir = "SUMMARY/VARY-P/"
out_template = out_dir + "{}-varyall{}.csv"

# param with rpi or nuc
zero_p_dir = results_dir + "kern-vs-user/2022-12-10T17.53.14+00.00/{}/tru-kxdp-0.5p-1ms/"
ps = ["0.01", "0.05", "0.1", "0.25", "0.5", "1"]
rates = ["0.1", "0.5", "1", "10", "50", "100"]
machines = ["rpi", "nuc"]

summary_format = "{}-summary-{}M.csv"

# heavy at:
other_p_dir = results_dir + "kxdp-blocking/2022-12-11T01.41.58+00.00/{}/tru-block-xdp-{}p-1ms/"

# uxdp at:
uxdp_p_dir = results_dir + "user-blocking/2022-12-15T09.32.58+00.00/{}/tru-block-user-100k-{}p-1ms/"

# what do I want?
# p, rate, pktsz, lat median, lat 99th, mean tput

def main():
	pathlib.Path(out_dir).mkdir(parents=True, exist_ok=True)

	for machine in machines:
		with open(out_template.format(machine, ""), "w") as of:
			build_outfile(machine, of)

		with open(out_template.format(machine, "-upcall"), "w") as of:
			build_outfile(machine, of, p_dir=uxdp_p_dir, excl_rates=["0.5", "50"], excl_szs=["128", "1024", "1280"])

def build_outfile(machine, file, do_zero_p=True, p_dir=other_p_dir, **kw_args):
	file.write("p,rate,pktsz,lat_median,lat_99th,mean_tput\n")

	if do_zero_p:
		do_one_p("0", zero_p_dir.format(machine), file, **kw_args)

	for p in ps:
		do_one_p(p, p_dir.format(machine, p), file, **kw_args)

def do_one_p(p, root, file, excl_rates=[], excl_szs=[]):
	for rate in rates:
		if rate in excl_rates:
			continue

		try:
			with open(root + summary_format.format("t", rate)) as tputs:
				with open(root + summary_format.format("l", rate)) as lats:
					for (t_row, l_row) in zip(tputs.readlines()[1:], lats.readlines()[1:]):
						l_splits = l_row.split(",")
						t_splits = t_row.split(",")

						# single-core, and trim.
						if l_splits[0] != "1" or l_splits[1] in excl_szs:
							continue

						#either(1), lat(6), lat(9), t(5)
						file.write(",".join([p, rate, l_splits[1],
							l_splits[6], l_splits[9],
							t_splits[5]]))
						file.write("\n")
		except:
			pass

if __name__ == "__main__":
	main()
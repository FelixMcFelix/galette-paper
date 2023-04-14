import csv
import math
import numpy as np
import os
import pathlib
import scipy.stats as st
import sys

sys.stdout.reconfigure(encoding='utf-8')

base_results_dir = "../results/SUMMARY/"
out_dir = "build/vs-baselines-new/"

dplanes_and_files = [
	{
		"title":        "Pure XDP",
		"irq_path":     "kern-vs-user-{0}/2023-01-05T09.50.11+00.00/{0}/tru-kxdp-0.5p-1ms/",
		"poll_path":    "poll-user-{0}/2023-01-07T08.42.01+00.00/{0}/tru-kxdp-0.5p-0ms/",
		"skip_for_rpi": False,
	},
	{
		"title":        "\\texttt{AF\\_XDP}",
		"irq_path":     "kern-vs-user-{0}/2023-01-05T09.50.11+00.00/{0}/tru-uxdp-0.5p-1ms/",
		"poll_path":    "poll-user-{0}/2023-01-07T08.42.01+00.00/{0}/tru-uxdp-0.5p-0ms/",
		"skip_for_rpi": False,
	},
	{
		"title":        "\\texttt{AF\\_PACKET}",
		"irq_path":     None,
		"poll_path":    "baseline-{1}/2023-01-05T09.50.11+00.00/{0}/testpmd-afp/",
		"skip_for_rpi": False,
	},
	{
		"title":        "DPDK",
		"irq_path":     None,
		"poll_path":    "baseline-{1}/2023-01-05T09.50.11+00.00/{0}/testpmd-dpdk/",
		"skip_for_rpi": True,
	},
]

rates = ["0.1", "1", "10", "50", "100", "1000"]
machines = ["nuc", "rpi"]
modes = ["irq", "poll"]
columns = [
	{
		"prefix": "t",
		"tex_label": "Throughput (\\unit{\\mega\\bit\\per\\second})",
		"pi_only": False,
		"indices": [5,8],
	},
	{
		"prefix": "c",
		"tex_label": "CPU (\\unit{\\percent})",
		"pi_only": False,
		"indices": [2, 4],
		"scale": 100.0
	},
	{
		"prefix": "p",
		"tex_label": "Power (\\unit{\\milli\\watt})",
		"pi_only": True,
		"indices": [2, 4],
	},
]

file_name_template = "{}-summary-{}M.csv"

def main():
	# load all files in.
	pkt_szs = set()

	for dplane in dplanes_and_files:
		for machine in machines:
			if dplane["skip_for_rpi"] and machine == "rpi":
				continue

			for mode in modes:
				src_path = mode + "_path"
				if dplane[src_path] is None:
					continue

				machine_alter = "pi" if machine == "rpi" else machine
				mach_path = dplane[src_path].format(machine, machine_alter)

				for col in columns:
					if col["pi_only"] and machine == "nuc":
						continue

					prefix = col["prefix"]

					dst_path = machine + "_" + mode + "_data_" + prefix

					dplane[dst_path] = {}

					for rate in rates[:-1] if machine == "rpi" else rates:
						with open(base_results_dir + mach_path + file_name_template.format(prefix, rate)) as of:
							lines = of.readlines()
							dat = [[float(v) for v in row.split(",")] for row in lines[1:]]

							for line in lines[1:]:
								pkt_szs.add(int(line.split(",")[1]))

							dplane[dst_path][rate] = np.array(dat)

	pathlib.Path(out_dir).mkdir(parents=True, exist_ok=True)

	for sz in pkt_szs:
		for machine in machines:
			with open(out_dir + "{}-{}B.tex".format(machine, sz), "w") as of:
				make_table(sz, machine, dplanes_and_files, of)

			if machine == "rpi":
				with open(out_dir + "{}-{}B-ex100.tex".format(machine, sz), "w") as of:
					make_table(sz, machine, dplanes_and_files, of, excl_100mb=True)

def make_table(packet_sz, machine, dplane_data, file, excl_100mb=False):
	l_columns = [c for c in columns if (not c["pi_only"]) or machine == "rpi"]

	n_cols = 2 * len(l_columns)
	table_spec = "@{}cc" + ("S[table-format=2.1(3)]" * n_cols) + "@{}"

	file.write(R"\begin{tabular}{" + table_spec + "}\n")

	file.write(R"\toprule\multicolumn{1}{c}{Dataplane} & \multicolumn{1}{c}{Ingest Rate (\unit{\mega\bit\per\second})}")
	for col in l_columns:
		file.write(R" & \multicolumn{2}{c}{" + col["tex_label"] + "}")
	file.write(R"\\" + "\n")

	for (i, col) in enumerate(l_columns):
		file.write(R"\cmidrule(lr){" + str(3 + 2*i) + "-" + str(4 + 2*i) + "}")

	# file.write(R" & \multicolumn{1}{c}{Throughput (\unit{\mega\bit\per\second})}")
	file.write(R" &")
	for col in l_columns:
		file.write(R" & \multicolumn{1}{c}{IRQ} & \multicolumn{1}{c}{Poll}")
	file.write(R"\\ \midrule" + "\n")

	for j, dset in enumerate(dplane_data):
		if j == 2:
			file.write(R"\cmidrule(lr){1-" + str(n_cols + 2) + "}")

		if dset["skip_for_rpi"] and machine == "rpi":
			continue

		for (i, rate) in enumerate(rates[:-1] if machine == "rpi" else rates):
			if (excl_100mb and rate in ["100", "1000"]) or (machine == "rpi" and rate == "1000"):
				continue

			file.write("" if i != 0 else dset["title"])
			file.write(" & " + rate)

			for col in l_columns:
				for mode in modes:
					lookup = machine + "_" + mode + "_data_" + col["prefix"]
					file.write(R" & ")
					if lookup not in dset:
						file.write(R"\multicolumn{1}{c}{---}")
					else:
						data = dset[lookup][rate]

						filt = [int(row[1]) == packet_sz for row in data]
						my_row = data[filt][0][col["indices"]]

						if "scale" in col:
							my_row = my_row * col["scale"]

						file.write("{:.1f}".format(my_row[0]))

						if len(my_row) >= 2:
							file.write("\\pm {:.1f}".format(my_row[1]))
							
			file.write("\\\\\n")


	# can use for a break if needed.
	# print(R"\cmidrule(lr){1-8}")

	file.write("\\bottomrule\n")
	file.write("\\end{tabular}\n")

if __name__ == "__main__":
	main()
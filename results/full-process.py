import csv
import math
import numpy as np
import os
import pathlib
import scipy.stats as st

exclude_dirs = set(["2022-11-29", "uxdp", "kxdp", "SUMMARY"])

prefix_to_type = {"t": "tput", "l": "lat", "c": "cpu", "p": "power"}

test_len = 11.0 # secs

file_name_template = "{}-{}B-{}M-{}C-{}.dat"
file_name_template_csv = "{}-{}B-{}M-{}C-{}.csv"

def process(root, files):
	# root is the dir, files are all the file names (no path)
	# GOAL:
	#  latency---cat all latencies, select percentiles, medians
	#  throughput---compute for known dt, output median, average as SUMMARY files.
	#  cpu---parse cpuinfo records, prune, 
	#  power (if present)---prune results at start & end, 

	core_cts = set()
	iter_max = 0
	pkt_szs = set()
	rates = set()

	# split files into categorisations.

	file_cats = {}
	for file in files:

		if "summary" in file:
			continue

		(prefix, sz, rate, core_ct, this_iter) = params_from_fname(file)

		if prefix in prefix_to_type:
			cat = prefix_to_type[prefix]
			if cat not in file_cats:
				file_cats[cat] = []
			file_cats[cat].append(file)

		pkt_szs.add(sz)
		rates.add(rate) # convert to float later, don't want representation issues to crop up.
		core_cts.add(core_ct)
		iter_max = max(iter_max, this_iter)

	if "lat" not in file_cats:
		print("WARN: dir", root, "has no latency results---skipping.")
		return

	shared_state = {
		"core_cts": sorted(core_cts),
		"iter_max": iter_max,
		"pkt_szs": sorted(pkt_szs),
		"rates": rates
	}

	print(root, ":", [(k, len(v)) for (k, v) in file_cats.items()])

	out_root = "SUMMARY/" + root
	pathlib.Path(out_root).mkdir(parents=True, exist_ok=True)

	process_latencies(root, shared_state, out_root)
	process_throughputs(root, shared_state, out_root)
	process_cpus(root, shared_state, out_root)

	if "power" in file_cats:
		process_powers(root, shared_state, out_root)

# main processing

def process_latencies(root, state, out_dir):
	for rate in state["rates"]:
		f_rate = float(rate)

		with open(out_dir + "/" + "l-summary-{}M.csv".format(rate), "w", newline="", encoding="utf-8") as out_file:
			writer = csv.writer(out_file, delimiter=',')
			writer.writerow([
				"cores", "pkt_sz",
				"lat_mean",
				"lat_min", "lat_10th", "lat_25th",
				"lat_median", "lat_75th", "lat_90th", "lat_99th",
				"lat_max",
			])

			for pkt_sz in state["pkt_szs"]:
				for core_ct in state["core_cts"]:
					lats = []

					for i in range(state["iter_max"]):
						fname = root + "/" + file_name_template.format(
							"l", pkt_sz, rate, core_ct, i
						)
						try:
							with open(fname) as in_file:
								lines = in_file.readlines()
								lats.extend([float(x) for x in lines[1:]])
						except FileNotFoundError:
							pass

					# print(len(lats))
					if len(lats) == 0:
						lats = [-1.0]

					writer.writerow([
						core_ct, pkt_sz,
						np.mean(lats),
						np.min(lats), np.percentile(lats, 10.0), np.percentile(lats, 25.0),
						np.median(lats), np.percentile(lats, 75.0), np.percentile(lats, 90.0), np.percentile(lats, 99.0),
						np.max(lats)
					])

def process_throughputs(root, state, out_dir):
	for rate in state["rates"]:
		f_rate = float(rate)

		with open(out_dir + "/" + "t-summary-{}M.csv".format(rate), "w", newline="", encoding="utf-8") as out_file:
			writer = csv.writer(out_file, delimiter=',')
			writer.writerow([
				"cores", "pkt_sz",
				"target_rate", "target_pps",
				"rate_median", "rate_mean",
				"pps_median", "pps_mean",
				"rate_95ci", "pps_95ci",
			])

			for pkt_sz in state["pkt_szs"]:
				for core_ct in state["core_cts"]:

					needed_pps = ((f_rate * 1.0e6) / 8.0) / float(pkt_sz)

					mbps_cell = []
					pps_cell = []

					for i in range(state["iter_max"]):
						fname = root + "/" + file_name_template.format(
							"t", pkt_sz, rate, core_ct, i
						)
						try:
							with open(fname) as in_file:
								lines = in_file.readlines()
								ibytes = extract_from_lua_line(lines[2])
								ierrors = extract_from_lua_line(lines[3])
								imissed = extract_from_lua_line(lines[4])
								ipackets = extract_from_lua_line(lines[5])
								obytes = extract_from_lua_line(lines[6])
								opackets = extract_from_lua_line(lines[8])

								if obytes == 0:
									# One or two really anomalous results...
									continue

								byps = float(ibytes) / test_len
								mbps = (byps * 8.0) / 1.0e6
								pps = float(ipackets) / test_len

								mbps_cell.append(mbps)
								pps_cell.append(pps)
						except FileNotFoundError:
							pass

					writer.writerow([
						core_ct, pkt_sz,
						rate, needed_pps,
						np.median(mbps_cell), np.mean(mbps_cell),
						np.median(pps_cell), np.mean(pps_cell),
						2.0 * np.std(mbps_cell), 2.0 * np.std(pps_cell)
					])

def process_powers(root, state, out_dir):
	for rate in state["rates"]:
		f_rate = float(rate)

		with open(out_dir + "/" + "p-summary-{}M.csv".format(rate), "w", newline="", encoding="utf-8") as out_file:
			writer = csv.writer(out_file, delimiter=',')
			writer.writerow([
				"cores", "pkt_sz",
				"mwatt_mean", "mwatt_stdev", "mwatt_95ci",
				"mwatt_median",
			])

			for pkt_sz in state["pkt_szs"]:
				for core_ct in state["core_cts"]:
					powers = []

					for i in range(state["iter_max"]):
						fname = root + "/" + file_name_template_csv.format(
							"p", pkt_sz, rate, core_ct, i
						)

						# Some of these might be missing due to power setup shenanigans.
						try:
							with open(fname) as in_file:
								lines = in_file.readlines()
								# print(lines[1])
								l_powers = [float(x.split(",")[2]) for x in lines[1 + 40:-10]]
								# seems to be a major issue in some files.
								# don't know if serial channel got misaligned?
								powers.extend([power for power in l_powers if power < 1000000])
						except FileNotFoundError:
							pass

					mean = np.mean(powers)
					ci = (st.t.interval(0.95, len(powers)-1, loc=mean, scale=st.sem(powers))[1] - mean)/ 2.0 if len(powers) >= 2 else -999.0
					writer.writerow([
						core_ct, pkt_sz,
						mean, np.std(powers), ci,
						np.median(powers),
					])

def process_cpus(root, state, out_dir):
	for rate in state["rates"]:
		f_rate = float(rate)

		with open(out_dir + "/" + "c-summary-{}M.csv".format(rate), "w", newline="", encoding="utf-8") as out_file:
			writer = csv.writer(out_file, delimiter=',')
			writer.writerow([
				"cores", "pkt_sz",
				"total_mean", "total_stdev", "total_95ci", "total_median",
				"user_mean", "user_stdev", "user_95ci", "user_median",
				"system_mean", "system_stdev", "system_95ci", "system_median",
			])

			for pkt_sz in state["pkt_szs"]:
				for core_ct in state["core_cts"]:
					cpu_utils = []

					for i in range(state["iter_max"]):
						fname = root + "/" + file_name_template.format(
							"c", pkt_sz, rate, core_ct, i
						)

						# Some of these might be missing due to power setup shenanigans.
						try:
							with open(fname) as in_file:
								content = in_file.read()
								blocks = content.split("\n\n")

								# print(len(blocks))

								# Measured every 0.5s
								blocks_trim = blocks[10 + 15:][:20]
								# blocks_trim = blocks

								measure_rows = [[int(count) for count in block.split("\n")[0].split(" ")[2:]] for block in blocks_trim]
								totals = [sum(row) for row in measure_rows]

								# want to skip any 0 deltas
								# try it with stride of 1 for now; cook up a better soln if needed.
								to_use = []
								for i, (t_0, t_1) in enumerate(zip(totals[0:], totals[1:])):
									if t_1 > t_0:
										to_use.append(i)

								for prev, curr in zip(to_use[0:], to_use[1:]):
									delta_total = float(totals[curr] - totals[prev])
									delta_user = float(measure_rows[curr][0] - measure_rows[prev][0])
									delta_system = float(measure_rows[curr][2] - measure_rows[prev][2])
									delta_idle = float(measure_rows[curr][3] - measure_rows[prev][3])

									cpu_utils.append([
										1.0 - (delta_idle / delta_total),
										delta_user / delta_total,
										delta_system / delta_total
									])

								# if "poll-user" in root and pkt_sz == 64:
								# 	print(i, cpu_utils[:-len(measure_rows)])
						except FileNotFoundError:
							pass

					cpu_utils = np.array(cpu_utils)

					means = np.mean(cpu_utils, axis=0)
					medians = np.median(cpu_utils, axis=0)
					stds = np.std(cpu_utils, axis=0)
					cis = 2.0 * stds

					# if "poll-user" in root and pkt_sz == 64:
					# 	print(means)
					# 	print(stds)
					# 	print(cis)

					row = np.array([means, stds, cis, medians])
					if np.isnan(row[0]).any():
						row = np.array([[-1.0, -1.0, -1.0, -1.0] for el in row])

					try:
						writer.writerow([core_ct, pkt_sz] + list(row[:,0]) + list(row[:,1]) + list(row[:,2]))
					except TypeError:
						print("AAAA", row)

# util fns
def extract_from_lua_line(line):
	return float(line.split(",")[0].split("=")[1].strip())

def params_from_fname(name):
	splits = name.split("-")

	sz = int(splits[1][:-1])
	rate = splits[2][:-1]
	core_ct = int(splits[3][:-1])

	this_iter = int(splits[4].split(".")[0])

	return (splits[0], sz, rate, core_ct, this_iter)

# main

def main():
	# walk to every *leaf* directory, exclude top-level dirs
	for i, (root, dirs, files) in enumerate(os.walk("./")):

		# prune only TOP-level dirs.
		if i == 0:
			dirs[:] = [d for d in dirs if d not in exclude_dirs]

		if len(dirs) == 0 and len(files) != 0:
			process(root, files)

if __name__ == "__main__":
	main()
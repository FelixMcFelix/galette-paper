import math
import numpy as np
import scipy.stats as spss

# pkt sz, stress level, iter
# TODO: do this for all dates.
stress_file_tpu = "2022-11-29/t-{}B-{}M-1C-{}-{}.dat"

histo_bin_width = 200

dplanes = ["kxdp-rpi", "uxdp-rpi"];
pkt_sizes = [64, 128, 256, 512, 1024, 1280, 1518];
iter_ct = 1
rates = [0.1, 0.5, 1, 10, 50, 100]

# { {
#     ibytes = 21536748776,
#     ierrors = 0,
#     imissed = 123196123,
#     ipackets = 21116998,
#     obytes = 147196761216,
#     oerrors = 0,
#     opackets = 144313056,
#     rx_nombuf = 0
#   },
#   n = 1
# }

def extract_from_line(line):
	return float(line.split(",")[0].split("=")[1].strip())

for rate in rates:
	for dplane in dplanes:
		# make file here
		with open("2022-11-29/t-summary-{}M-{}".format(rate, dplane), "w") as wfile:
			out_lines = []
			for pkt_size in pkt_sizes:
				for i in range(iter_ct):
					with open(stress_file_tpu.format(pkt_size, rate, i, dplane)) as of:
						lines = of.readlines()
						ibytes = extract_from_line(lines[2])
						ierrors = extract_from_line(lines[3])
						imissed = extract_from_line(lines[4])
						ipackets = extract_from_line(lines[5])
						obytes = extract_from_line(lines[6])
						opackets = extract_from_line(lines[8])

						t = 11.0
						byps = float(ibytes) / t
						mbps = (byps * 8.0) / 1.0e6
						pps = float(ipackets) / t
						needed_pps = ((float(rate) * 1.0e6) / 8.0) / pkt_size

						out_lines.append("{} {} {} {}\n".format(pkt_size, mbps, pps, needed_pps))

			wfile.writelines(out_lines)
			
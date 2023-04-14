set datafile separator ","

set xlabel "Packet Size (\\unit{\\byte})"
set ylabel "Macswap Latency (\\unit{\\micro\\second})"

set key textcolor rgb "black"
set tics textcolor rgb "black"
set label textcolor rgb "black"

set tics nomirror

set logscale y
set grid y noxtics

set key samplen 2 spacing 1 width -2 above

src_files = 5

# Example results files: (64 vs 1518B are on different rows)
array PktSizeLines[2]
PktSizeLines[1] = 1 #64
PktSizeLines[2] = 7 #1518

array SrcFiles[5]
SrcFiles[1] = "../results/SUMMARY/kern-vs-user/2022-12-10T17.53.14+00.00/rpi/tru-kxdp-0.5p-1ms/l-summary-50M.csv"
SrcFiles[2] = "../results/SUMMARY/kern-vs-user/2022-12-10T17.53.14+00.00/rpi/tru-uxdp-0.5p-1ms/l-summary-50M.csv"
SrcFiles[3] = "../results/SUMMARY/poll-user/2022-12-21T11.43.12+00.00/rpi/tru-uxdp-0.5p-0ms/l-summary-50M.csv"
SrcFiles[4] = "../results/SUMMARY/baseline-pi/2022-12-20T21.49.20+00.00/rpi/testpmd-afp/l-summary-50M.csv"
SrcFiles[5] = "../results/SUMMARY/poll-user/2022-12-21T11.43.12+00.00/rpi/tru-kxdp-0.5p-0ms/l-summary-50M.csv"

array Titles[5]
Titles[1] = "Pure XDP"
Titles[2] = "\\texttt{AF\\_XDP} (\\qty{1}{\\milli\\second})"
Titles[3] = "\\texttt{AF\\_XDP} (Polled)"
Titles[4] = "\\texttt{AF\\_PACKET}"
Titles[5] = "Pure XDP (Polled)"

array LineStyles[6]
LineStyles[1] = 3
LineStyles[2] = 4
LineStyles[3] = 5
LineStyles[4] = 6
LineStyles[5] = 7
LineStyles[6] = 8

array DashStyles[6]
DashStyles[1] = 1
DashStyles[2] = 2
DashStyles[3] = 3
DashStyles[4] = 4
DashStyles[5] = 5
DashStyles[6] = 6

# myTitle(i) = sprintf("%s Mbps", Rates[i + 1])
# singleTitle(i) = sprintf("\\emph{\\Indfw} (\\SI{%d}{\\bit})", BitDepths[i + 1])

file(i) = SrcFiles[(i % src_files)+1]

xcoord(i) = 1.0 + (i * 1.0) + floor((i * 1.0) / src_files)

row(i) = PktSizeLines[floor(i / src_files) + 1]

set bars 1.0
set boxwidth 1.0
set style fill empty

set xrange [0.0:(2.0 * (src_files + 1.0))]

set xtics ("64" 2.5, \
	"1518" 7.5\
	) scale 0.0

# candlesticks, THEN medians
# x coord, open, low, high, close
plot for [i=0:(src_files-1)] file(i) every ::row(i)::row(i) using (xcoord(i)):(1.0e-3 * $6):(1.0e-3 * $4):(1.0e-3 * $10):(1.0e-3 * $8) with candlesticks ls LineStyles[(i % src_files) + 1] fill pattern (i % src_files) title Titles[(i % src_files) + 1] whiskerbars 0.5, \
	for [i=src_files:((2 * src_files) - 1)] file(i) every ::row(i)::row(i) using (xcoord(i)):(1.0e-3 * $6):(1.0e-3 * $4):(1.0e-3 * $10):(1.0e-3 * $8) with candlesticks ls LineStyles[(i % src_files) + 1] fill pattern (i % src_files) notitle whiskerbars 0.5, \
	for [i=0:((2 * src_files) - 1)] file(i) every ::row(i)::row(i) using (xcoord(i)):(1.0e-3 * $7):(1.0e-3 * $7):(1.0e-3 * $7):(1.0e-3 * $7) with candlesticks lt -1 notitle

# plot for [i=0:5] file(i) u 1:3 with linespoints title myTitle(i) ls LineStyles[i + 1] dt DashStyles[1], \
# 	for [i=0:5] file(i) u 1:4 with linespoints ls LineStyles[i + 1] dt 2 notitle


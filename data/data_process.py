import os
import re
import csv

"""
A simple script that will take data from results-fwd-test and make a nicer CSV summary
that reflects how many BDD nodes were used in each iteration of the reachability benchmark.
"""

if not os.path.exists('./results-fwd-summarised'):
     os.makedirs('./results-fwd-summarised')

for result_file in os.listdir('./results-fwd-test'):
	if not result_file.endswith('.txt'):
		continue	
	with open(f"./results-fwd-test/{result_file}") as file:		
		print(f"Loaded result file: {result_file}")
		step = 0	
		completed_iterations = 0	
		accumulated = []
		max_bdd_size = [0,0,0,0]
		for line in file:
			if line.startswith('Completed one wave'):
				# Restart once new wave is found.
				step = 0
				completed_iterations += 1
				continue

			step_line = re.match(" \\> BDD sizes\\: Some\\((\d+)\\) Some\\((\d+)\\) Some\\((\d+)\\) Some\\((\d+)\\)", line)
			if step_line is not None:
				# Found one step line.				
				if len(accumulated) <= step:
					# reached_by, size_1, .. size_4
					accumulated.append([0,0,0,0,0])
				line_sizes = [int(step_line[i + 1]) for i in range(4)]
				max_bdd_size = [max(max_bdd_size[i], line_sizes[i]) for i in range(4)]
				accumulated[step][0] += 1
				accumulated[step][1] += line_sizes[0]
				accumulated[step][2] += line_sizes[1]
				accumulated[step][3] += line_sizes[2]
				accumulated[step][4] += line_sizes[3]
				step += 1
		print(f"  >> Fully completed iterations: {completed_iterations}")
		print(f"  >> Max. iterations lnegth: {len(accumulated)}")
		print(f"  >> Max. BDD size: {max_bdd_size}")
		with open(f"./results-fwd-summarised/{result_file.replace('.txt', '.csv')}", 'w') as outfile:
			writer = csv.writer(outfile)
			writer.writerow(['iteration', 'unary', 'binary', 'gray', 'petri'])
			for row in accumulated:
				writer.writerow(row)
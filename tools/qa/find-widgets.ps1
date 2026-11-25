function Walk($n){ if($n.name -and ($n.name -match '全选当前结果|退出批量|清空')){ "{0} | {1} | {2}" -f $n.ref_id,$n.role,$n.name }; foreach($c in $n.children){ Walk $c } }
$j=Get-Content D:\Workspace\_s.json -Raw | ConvertFrom-Json
Walk $j.data.tree

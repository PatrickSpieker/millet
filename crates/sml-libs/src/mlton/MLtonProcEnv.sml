signature MLTON_PROC_ENV = sig
  type gid
  val setenv : {name : string, value : string} -> unit
  val setgroups : gid list -> unit
end

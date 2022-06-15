signature NET_PROT_DB (* OPTIONAL *) = sig
  type entry
  val name : entry -> string
  val aliases : entry -> string list
  val protocol : entry -> int
  val getByName : string -> entry option
  val getByNumber : int -> entry option
end

structure NetProtDB :> NET_PROT_DB (* OPTIONAL *) = struct end

///////////////////////
///  Empty modules  ///
///////////////////////

module A;
	B b();
endmodule

module B;
endmodule

//@ elab A
//| entity @B () () {
//| }
//|
//| entity @A () () {
//|     inst @B () ()
//| }

//////////////////////////////////
///  Type parameter overrides  ///
//////////////////////////////////

module C;
	D #(void) d1();
	D #(bit) d2();
endmodule

module D #(type T) (input T t);
endmodule

//@ elab C
//| entity @D.param1 (void$ %t) () {
//| }
//|
//| entity @D.param2 (i1$ %t) () {
//| }
//|
//| entity @C () () {
//|     inst @D.param1 () ()
//|     inst @D.param2 () ()
//| }

//////////////////////////////////////
///  Default type parameter types  ///
//////////////////////////////////////

module E #(type T = bit) (input T t);
endmodule

//@ elab E
//| entity @E (i1$ %t) () {
//| }

//////////////////////////////////////////////
///  Dependencies between type parameters  ///
//////////////////////////////////////////////

module F;
	G #(bit) g1();
	G #(void) g2();
	G #(void, bit) g3();
endmodule

module G #(type T, type R = T) (input T t, input R r);
endmodule

//@ elab F
//| entity @G.param1 (i1$ %t, i1$ %r) () {
//| }
//|
//| entity @G.param2 (void$ %t, void$ %r) () {
//| }
//|
//| entity @G.param3 (void$ %t, i1$ %r) () {
//| }
//|
//| entity @F () () {
//|     inst @G.param1 () ()
//|     inst @G.param2 () ()
//|     inst @G.param3 () ()
//| }

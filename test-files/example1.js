// Example JavaScript file with dead code

function usedFunction() {
    console.log("This function is used");
}

function unusedFunction() {
    console.log("This function is never called");
}

const usedVariable = "This is used";
const unusedVariable = "This is never used";

let anotherUnusedVar = 42;
let usedVar = 10;

// Using some variables
console.log(usedVariable);
console.log(usedVar);

// Call the used function
usedFunction();

// Unused class
class UnusedClass {
    constructor() {
        this.value = 0;
    }
    
    method() {
        return this.value;
    }
}

// Used class
class UsedClass {
    constructor(name) {
        this.name = name;
    }
}

const instance = new UsedClass("test");

// TypeScript example with dead code

interface UnusedInterface {
    id: number;
    name: string;
}

interface UsedInterface {
    value: string;
}

function processData(data: UsedInterface): string {
    return data.value.toUpperCase();
}

const deadVariable: string = "never used";
const liveVariable: number = 42;

// Some unused imports would go here
// import { unusedImport } from 'some-module';

// Unused arrow function
const unusedArrowFunction = (x: number) => x * 2;

// Used arrow function  
const usedArrowFunction = (x: number) => x + 1;

// Dead code after return
function functionWithDeadCode() {
    return "early return";
    
    // This code is unreachable
    const deadAfterReturn = "unreachable";
    console.log(deadAfterReturn);
}

// Usage
console.log(processData({ value: "test" }));
console.log(usedArrowFunction(liveVariable));
functionWithDeadCode();
